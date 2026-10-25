# Bilibili API 接口清单

本文档整理 BiliMusic 当前使用到的 Bilibili 接口，并用 `curl` 展示等价请求方式。

> 说明：接口请求示例中的 `<...>` 是需要替换的变量。请使用自己的账号 Cookie，不要把 Cookie、`SESSDATA`、`bili_jct` 或完整签名提交到仓库。

## 1. 请求约定

### 1.1 基础地址

```text
https://api.bilibili.com
```

登录二维码接口使用：

```text
https://passport.bilibili.com
```

字幕文件通常来自 `aisubtitle.hdslb.com` 或其他 Bilibili/HDSlb CDN 地址。

### 1.2 推荐请求头

项目的渲染进程请求会携带登录态 Cookie 和 Referer：

```bash
export BILI_COOKIE='SESSDATA=<your-sessdata>; bili_jct=<your-bili-jct>; DedeUserID=<your-mid>'

curl 'https://api.bilibili.com/...' \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

部分接口还会检查 `Origin`：

```bash
curl 'https://api.bilibili.com/...' \
  -H 'Origin: https://www.bilibili.com' \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

### 1.3 WBI 签名接口

以下接口在项目中通过 WBI 签名调用：

- 视频搜索：`/x/web-interface/wbi/search/type`
- UP 主搜索：`/x/web-interface/wbi/search/type`
- UP 主投稿：`/x/space/wbi/arc/search`
- 播放器信息和字幕列表：`/x/player/wbi/v2`

WBI 请求不能只照抄普通 Query 参数。需要：

1. 请求 `/x/web-interface/nav` 获取 `wbi_img.img_url` 和 `wbi_img.sub_url`。
2. 从两个 URL 中取文件名作为 `img_key` 和 `sub_key`。
3. 按 Bilibili 的固定置换表生成 `mixinKey`。
4. 将业务参数加上当前 Unix 时间戳 `wts`。
5. 按键名排序、URL 编码、拼接 `w_rid=md5(query + mixinKey)`。

curl 示例中的 `<signed-query>` 表示已经完成上述步骤的 Query 字符串。

---

## 2. 用户与登录状态

### 2.1 获取当前用户信息 / 登录状态

**接口：** `GET /x/web-interface/nav`

**项目用途：**

- 判断是否登录。
- 获取当前用户 UID、昵称和头像。
- 获取 WBI 签名所需的 `wbi_img` 密钥来源。

```bash
curl 'https://api.bilibili.com/x/web-interface/nav' \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

匿名请求也可能返回基础数据，但登录字段可能不可用或接口返回登录错误码。

### 2.2 获取 WBI 密钥

项目没有单独封装一个“WBI 密钥接口”，而是复用上面的 nav 接口：

```bash
curl 'https://api.bilibili.com/x/web-interface/nav' \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}" \
  | jq '.data.wbi_img'
```

返回数据中重点关注：

```json
{
  "img_url": "https://i0.hdslb.com/bfs/wbi/<img-key>.png",
  "sub_url": "https://i0.hdslb.com/bfs/wbi/<sub-key>.png"
}
```

### 2.3 生成扫码登录二维码

**接口：** `GET https://passport.bilibili.com/x/passport-login/web/qrcode/generate`

**项目用途：** 登录弹窗生成二维码。

```bash
curl 'https://passport.bilibili.com/x/passport-login/web/qrcode/generate' \
  -H 'Referer: https://passport.bilibili.com/login'
```

返回值中的：

- `data.url`：二维码内容。
- `data.qrcode_key`：后续轮询使用的临时 Key。

### 2.4 轮询扫码登录状态

**接口：** `GET /x/passport-login/web/qrcode/poll`

```bash
export QRCODE_KEY='<qrcode_key>'

curl "https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key=${QRCODE_KEY}" \
  -H 'Referer: https://passport.bilibili.com/login' \
  -i
```

常见状态由响应中的状态码表示：

- `86101`：二维码未扫码。
- `86090`：已扫码，等待确认。
- `0`：登录成功。

Electron 主进程使用手动重定向模式读取登录成功时的 `Set-Cookie`，再写入 Bilibili Cookie。

### 2.5 退出登录

退出登录没有调用 Bilibili 的远程注销接口。项目直接清除 Electron Session 中的 Cookie：

- `SESSDATA`
- `bili_jct`
- `DedeUserID`
- `DedeUserID__ckMd5`

---

## 3. 视频搜索与详情

### 3.1 搜索视频

**接口：** `GET /x/web-interface/wbi/search/type`

**项目用途：** 搜索页面和搜索结果中的视频列表。

参数：

- `search_type=video`
- `keyword`：搜索关键词。
- `page`：页码。
- `page_size`：每页数量。
- `wts`、`w_rid`：WBI 签名参数。

```bash
curl "https://api.bilibili.com/x/web-interface/wbi/search/type?<signed-query>" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

未签名的等价参数形态如下，仅用于说明业务参数：

```text
search_type=video&keyword=<url-encoded-keyword>&page=1&page_size=20
```

Electron 主进程中还保留了一个早期 IPC handler，使用旧式未签名路径：

```bash
curl "https://api.bilibili.com/x/web-interface/search/type?search_type=video&keyword=<url-encoded-keyword>&page=1&pagesize=20" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

当前渲染进程搜索优先使用 WBI 版本；旧式路径可能受到接口风控限制。

结果中的常用字段：

- `bvid`
- `aid`
- `title`
- `author`
- `duration`
- `pic`
- `play`

### 3.2 获取视频详情

**接口：** `GET /x/web-interface/view`

**项目用途：**

- 获取视频标题、作者、封面和总时长。
- 获取默认 `cid`。
- 获取所有分P的 `pages`、分P标题、分P时长和分P `cid`。
- 获取评论接口需要的 `aid`。

```bash
export BVID='<video-bvid>'

curl "https://api.bilibili.com/x/web-interface/view?bvid=${BVID}" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

多P视频的关键字段：

```json
{
  "bvid": "<video-bvid>",
  "aid": 123456,
  "cid": 111111,
  "duration": 3600,
  "videos": 3,
  "pages": [
    {
      "cid": 111111,
      "page": 1,
      "part": "第一部分",
      "duration": 1200
    }
  ]
}
```

播放、字幕和进度条必须使用同一个分P的 `cid`，不能只使用视频默认 `cid`。

### 3.3 搜索 UP 主

**接口：** `GET /x/web-interface/wbi/search/type`

**项目用途：** 搜索页面中的 UP 主搜索。

业务参数：

- `search_type=bili_user`
- `keyword`
- `page`
- `page_size`
- `wts`、`w_rid`

```bash
curl "https://api.bilibili.com/x/web-interface/wbi/search/type?<signed-query>" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

### 3.4 获取 UP 主投稿视频

**接口：** `GET /x/space/wbi/arc/search`

**项目用途：** UP 主空间页面加载投稿列表。

业务参数：

- `mid`：UP 主 UID。
- `pn`：页码。
- `ps`：每页数量。
- `order=pubdate` 或 `order=click`。
- `platform=web`。
- `wts`、`w_rid`。

```bash
curl "https://api.bilibili.com/x/space/wbi/arc/search?<signed-query>" \
  -H 'Referer: https://space.bilibili.com/<mid>/video' \
  -H "Cookie: ${BILI_COOKIE}"
```

---

## 4. 播放、音频与字幕

### 4.1 获取播放器音频流

**接口：** `GET /x/player/playurl`

**项目用途：** 获取 DASH 音频流地址。

参数：

- `bvid`
- `cid`：当前视频或当前分P的 CID。
- `qn=0`：请求最高可用质量。
- `fnver=0`
- `fnval=16`：请求 DASH 数据。
- `fourk=1`

```bash
curl "https://api.bilibili.com/x/player/playurl?bvid=${BVID}&cid=<cid>&qn=0&fnver=0&fnval=16&fourk=1" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

项目从响应中的 `data.dash.audio` 选择音频流，主要使用：

- `id`
- `bandwidth`
- `mimeType`
- `baseUrl`
- `backupUrl`

`baseUrl` 和 `backupUrl` 是带临时签名的 CDN 地址，不应长期保存或提交到仓库。

### 4.2 播放器信息与字幕列表

**接口：** `GET /x/player/wbi/v2`

**项目用途：** 获取 Bilibili 页面中的 CC 字幕和 AI 字幕列表。

业务参数：

- `bvid`
- `cid`
- `wts`、`w_rid`

```bash
curl "https://api.bilibili.com/x/player/wbi/v2?<signed-query>" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

返回中的字幕列表可能出现在：

- `data.subtitle.subtitles`
- `data.subtitle.list`

项目会合并两个字段、去重，并优先选择中文 AI 字幕。字幕项通常包含：

- `id`
- `lan`
- `lan_doc`
- `subtitle_url`
- `ai_status`
- `ai_type`

### 4.3 获取字幕文件

播放器接口返回 `subtitle_url` 后，项目再请求字幕 CDN JSON 文件。

```bash
export SUBTITLE_URL='<subtitle_url>'

curl "${SUBTITLE_URL}" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

字幕响应通常包含：

```json
{
  "body": [
    {
      "from": 0.5,
      "to": 3.2,
      "content": "歌词或字幕文本"
    }
  ]
}
```

Electron 渲染进程不能直接稳定读取字幕 CDN，因此项目通过主进程 `net.fetch` 请求，并限制 URL 只能是 HTTPS 的 Bilibili 或 HDSlb 域名。

### 4.4 下载音频

下载不是一个新的 Bilibili API。项目先调用播放器接口取得 `baseUrl`，再由 Electron 主进程下载这个临时 CDN 地址：

```bash
curl -L "<audio_base_url>" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}" \
  -o output.m4s
```

实际项目中下载由 Electron `net.fetch` 完成，并保存到应用用户目录下的 `downloads` 文件夹。

---

## 5. 评论

### 5.1 获取视频评论

**接口：** `GET /x/v2/reply`

**项目用途：** 播放页评论面板。

参数：

- `type=1`：视频评论。
- `oid`：视频的 `aid`，不是 `bvid`。
- `pn`：页码。
- `ps`：每页数量。
- `sort=2`：按项目当前实现使用的排序方式。

```bash
export AID='<video-aid>'

curl "https://api.bilibili.com/x/v2/reply?type=1&oid=${AID}&pn=1&ps=20&sort=2" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

项目会把返回的 `replies` 转换为：

- 评论 ID。
- 用户名和头像。
- 评论内容。
- 点赞数。
- 回复数。
- 发布时间。

---

## 6. 推荐、热门与音乐列表

### 6.1 热门视频

**接口：** `GET /x/web-interface/popular`

```bash
curl 'https://api.bilibili.com/x/web-interface/popular?ps=10&pn=1' \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

参数：

- `ps`：每页条数。
- `pn`：页码。

### 6.2 个性化推荐

**接口：** `GET /x/web-interface/index/top/rcmd`

```bash
curl 'https://api.bilibili.com/x/web-interface/index/top/rcmd?ps=20' \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

参数：

- `ps`：请求条数。

### 6.3 音乐区排行榜

**接口：** `GET /x/web-interface/ranking/region`

```bash
curl 'https://api.bilibili.com/x/web-interface/ranking/region?rid=3&day=3' \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

项目使用：

- `rid=3`：音乐区。
- `day=3`：接口请求的时间范围参数。

### 6.4 音乐中心综合榜

**接口：** `GET /x/centralization/interface/music/comprehensive/web/rank`

```bash
curl 'https://api.bilibili.com/x/centralization/interface/music/comprehensive/web/rank?pn=1&ps=30' \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

该接口中的部分顶层 `bvid` 是音乐中心元数据 ID，项目优先使用 `related_archive.bvid` 作为实际可播放稿件。

### 6.5 音乐中心新歌速递

**接口：** `GET /x/centralization/interface/new/music`

```bash
curl 'https://api.bilibili.com/x/centralization/interface/new/music' \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

项目使用返回的歌曲名称、作者、封面、专辑和关联稿件信息构造音乐列表。

---

## 7. 收藏夹

### 7.1 获取收藏夹列表

**接口：** `GET /x/v3/fav/folder/created/list-all`

```bash
export MID='<user-mid>'

curl "https://api.bilibili.com/x/v3/fav/folder/created/list-all?up_mid=${MID}" \
  -H 'Referer: https://www.bilibili.com/' \
  -H "Cookie: ${BILI_COOKIE}"
```

项目使用该接口获取收藏夹名称、ID 和媒体数量。当前代码没有通过该接口执行收藏或取消收藏操作。

---

## 8. 项目接口与远程接口的对应关系

项目在 Electron 中同时存在两条调用路径：

### 8.1 当前渲染进程路径

主要代码：

```text
src/services/bilibiliApi.ts
```

特点：

- 使用浏览器环境 `fetch`。
- 自动携带 Electron 页面 Cookie。
- WBI 接口由渲染进程签名并请求。
- 当前播放、分P详情和 AI 字幕主要走这条路径。

### 8.2 Electron IPC 路径

主要代码：

```text
electron/biliApi.ts
electron/preload.cjs
```

特点：

- 通过 `ipcRenderer.invoke()` 调用主进程。
- 主进程从 Electron Session 读取 Cookie。
- 字幕 CDN 和音频下载使用主进程网络能力。
- 文件中保留了若干早期的普通接口 IPC handler，例如视频搜索、视频详情和播放地址。

### 8.3 重要的 CID 规则

对视频播放和字幕来说，必须保证以下请求使用同一个 CID：

```text
视频详情 pages[].cid
        ↓
/x/player/playurl?cid=<当前分Pcid>
        ↓
/x/player/wbi/v2?cid=<当前分Pcid>
```

如果多P视频错误地使用默认 CID，可能出现：

- 播放了错误分P的音频。
- AI 字幕显示“暂无歌词”。
- 音频时长与进度条不一致。

---

## 9. 常见错误与排查

### 9.1 返回 `-101`

通常表示未登录或登录态不可用。检查：

```bash
curl 'https://api.bilibili.com/x/web-interface/nav' \
  -H "Cookie: ${BILI_COOKIE}" \
  -H 'Referer: https://www.bilibili.com/'
```

### 9.2 返回 `-352`

通常表示请求被风控拦截。优先检查：

- 是否使用了完整浏览器 Cookie。
- 是否携带 `Referer`。
- WBI 接口的签名 Query 是否和实际发送的 Query 完全一致。
- 是否把签名后的 Query 再次编码或改写。

### 9.3 播放地址能返回但播放卡住

检查：

1. `data.dash.audio` 是否为空。
2. 当前选择的音频流是否存在 `baseUrl`。
3. 是否携带了临时签名参数。
4. 是否需要尝试其他音频流或 `backupUrl`。
5. 当前音频流对应的 `cid` 是否属于当前分P。

### 9.4 字幕列表为空

检查：

1. 是否调用了带 WBI 签名的 `/x/player/wbi/v2`。
2. 是否使用了当前分P的 CID。
3. 是否同时读取了 `subtitle.subtitles` 和 `subtitle.list`。
4. `subtitle_url` 是否为空。
5. 字幕 CDN 请求是否携带 Bilibili Referer 和登录态。

---

## 10. 代码位置索引

| 功能 | 主要文件 |
|---|---|
| Bilibili 渲染进程 API | `src/services/bilibiliApi.ts` |
| API 适配层 | `src/services/api.ts` |
| Electron 主进程 API | `electron/biliApi.ts` |
| Electron preload IPC 暴露 | `electron/preload.cjs` |
| 播放器调用 | `src/contexts/PlayerContext.tsx` |
| Bilibili AI 字幕歌词转换 | `src/services/lyrics.ts` |
| 评论调用 | `src/services/api.ts`、`src/services/bilibiliApi.ts` |
| 登录状态 | `src/contexts/AuthContext.tsx`、`src/services/api.ts` |
