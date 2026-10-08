#!/bin/sh
# 仅由当前 Suno/Ego 控制者运行；复用已打开的原曲 Studio 页，不登录或调用下载 API。
set -eu
if [ "${1:-}" = "--version" ]; then
  echo "1.0.0"
  exit 0
fi
if [ "${1:-}" = "--help" ] || [ "$#" -lt 5 ] || [ "$#" -gt 6 ]; then
  cat <<'HELP'
Suno Studio 原生 WAV 导出助手 1.0.0
用法：sh scripts/export_studio.sh SPACE PAGE CLIP_SELECTOR TITLE OUTPUT.wav [DOWNLOAD_SELECTOR]

先在既有 Ego 登录会话正常打开目标歌曲：
歌曲菜单 → Edit → Open in Studio → Single-track / Use the full mix。
CLIP_SELECTOR 必须是当前已核对的唯一画布 CSS 选择器，不能使用旧快照编号。
运行前由浏览器控制者确认当前单轨 full mix 正是来源歌曲；标题存在不能代替该确认。
TITLE 为当前原曲标题；输出必须是新的绝对路径，已有文件一律保留。
DOWNLOAD_SELECTOR 默认为 loc=role:button[name='Download .WAV']。
波形在画布局部区域时，传入现场核对的 SUNO_EXPORT_CLIP_X / SUNO_EXPORT_CLIP_Y，
表示相对 CLIP_SELECTOR 左上角的 CSS 像素，不使用旧页面的屏幕坐标。
助手在同一调用中完成右键、等待下载、点击与保存，不生成、分轨、读取或导出凭据。
下载后使用 check_audio.py 核对完整时长；账号权益及额度仍须由页面前后核对。
HELP
  if [ "${1:-}" = "--help" ]; then exit 0; else exit 2; fi
fi
export SUNO_EXPORT_SPACE="$1"
export SUNO_EXPORT_PAGE="$2"
export SUNO_EXPORT_CLIP_SELECTOR="$3"
export SUNO_EXPORT_TITLE="$4"
export SUNO_EXPORT_OUTPUT="$5"
export SUNO_EXPORT_DOWNLOAD_SELECTOR="${6:-loc=role:button[name='Download .WAV']}"

ego-browser nodejs <<'JS'
const fs = await import("node:fs/promises");
const path = await import("node:path");
const crypto = await import("node:crypto");
const output = process.env.SUNO_EXPORT_OUTPUT;
const space = Number(process.env.SUNO_EXPORT_SPACE);
const pageLabel = process.env.SUNO_EXPORT_PAGE;
const clipSelector = process.env.SUNO_EXPORT_CLIP_SELECTOR;
  const title = process.env.SUNO_EXPORT_TITLE;
const clickPosition = process.env.SUNO_EXPORT_CLIP_X === undefined && process.env.SUNO_EXPORT_CLIP_Y === undefined
  ? undefined : { x: Number(process.env.SUNO_EXPORT_CLIP_X), y: Number(process.env.SUNO_EXPORT_CLIP_Y) };
let stage;
let saved = false;
try {
  if (!Number.isInteger(space) || space < 1 || !/^p\d+$/.test(pageLabel)) {
    throw new Error("需填写已有的 Space 编号和 Page 标签，不创建新会话");
  }
  if (!path.isAbsolute(output) || path.extname(output).toLowerCase() !== ".wav") {
    throw new Error("输出必须是原生 WAV 的绝对路径");
  }
  if (!clipSelector || !title || /^(?:@|ref=)/.test(clipSelector)) {
    throw new Error("需填写当前已核对的原曲标题与唯一选择器，不能用跨调用快照编号");
  }
  if (clickPosition && (!Number.isFinite(clickPosition.x) || !Number.isFinite(clickPosition.y) || clickPosition.x < 0 || clickPosition.y < 0)) {
    throw new Error("需填写现场核对的画布相对位置，不能猜测");
  }
  try {
    await fs.lstat(output);
    throw new Error("目标文件已存在，未操作浏览器，也未覆盖；请换输出文件名");
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  const task = await taskSpace(space);
  const page = task.page(pageLabel);
  const current = new URL(await page.url());
  if (current.origin !== "https://suno.com" || !current.pathname.startsWith("/studio")) {
    throw new Error("目标不是 Suno Studio 页，未操作");
  }
  if (!await page.evaluate(expected => document.body.innerText.includes(expected), title)) {
    throw new Error("当前页面未确认期望原曲标题，未操作");
  }
  const target = await page.evaluate(({ selector, position }) => {
    const nodes = [...document.querySelectorAll(selector.replace(/^loc=css:/, ""))];
    if (nodes.length !== 1 || nodes[0].tagName !== "CANVAS") return { uniqueCanvas: false };
    const rect = nodes[0].getBoundingClientRect();
    return { uniqueCanvas: true, visible: rect.width > 0 && rect.height > 0,
      initializing: document.body.innerText.includes("Initializing audio engine"),
      positionInside: !position || (position.x < rect.width && position.y < rect.height) };
  }, { selector: clipSelector, position: clickPosition });
  if (!target.uniqueCanvas || !target.visible || target.initializing || !target.positionInside) {
    throw new Error("当前页面未确认唯一已加载画布与有效波形位置，未操作");
  }
  await fs.mkdir(path.dirname(output), { recursive: true });
  stage = path.join(path.dirname(output), `.suno-export-${crypto.randomUUID()}.wav`);
  // 菜单与下载必须同轮完成，跨 Ego 调用会失去临时菜单。
  await page.click(clipSelector, { button: "right", position: clickPosition, label: "open original clip menu" });
  const pending = page.waitForEvent("download", { timeout: 45000 })
    .then(download => ({ download }), error => ({ error }));
  await page.click(process.env.SUNO_EXPORT_DOWNLOAD_SELECTOR, { label: "download original WAV" });
  const result = await pending;
  if (result.error) throw new Error("下载事件未完成；先核对现有结果再恢复，不自动重试");
  await result.download.saveAs(stage);
  const stat = await fs.stat(stage);
  if (!stat.isFile() || stat.size === 0) throw new Error("下载未取得有效文件");
  // 排他创建最终文件：即使其他操作刚创建了同名文件，也不会覆盖。
  await fs.link(stage, output);
  saved = true;
  console.log(JSON.stringify({ toolVersion: "1.0.0", status: "saved_unverified", space,
    page: pageLabel, expectedTitle: title, output, bytes: stat.size,
    entry: "original clip context menu > Download .WAV",
    next: "check_audio.py --expected-seconds；另核对账号前后额度" }));
} catch (error) {
  // 不输出 SDK 原始异常，避免意外带出临时下载链接或请求细节。
  const allowed = /^(?:需填写|输出必须|目标文件已存在|当前页面未确认|目标不是|下载事件未完成|下载未取得)/;
  console.log(JSON.stringify({ toolVersion: "1.0.0", status: "failed", output,
    reason: allowed.test(error.message) ? error.message : "导出操作未完成；由同一浏览器控制者核对页面和文件后恢复",
    finalFileCreated: saved }));
  process.exitCode = 1;
} finally {
  if (stage) await fs.unlink(stage).catch(() => {});
}
JS
