---
name: suno-native-download
description: Download existing, self-owned Suno songs as complete native audio, using the guarded Premier Studio tool when currently entitled, then verify files and separate download charges from generation charges.
metadata:
  version: "1.0.0"
---

# Suno 原生音频下载

用于下载用户自己已有的作品；不生成新音乐。先确认当前任务授权、曲目ID/完整标题、账号归属、会员权益及已有原件，避免重复下载。默认每首一份原生MP3；本包已验证的Studio路线**只支持原生WAV**，MP3不可用时用这一份备用格式，不传猜测的MP3参数、不调用普通Library授权、不转码或补多格式。

读取[安装和使用](references/usage.md)后调用随本skill安装的相对工具。预编译工具仅macOS Apple Silicon，版本`0.4.0-suno-studio.2`；它是独立便携修订，原下载合同来自已实测的`0.4.0-suno-studio.1`，新配置目录入口只经离线验证，同事首次实际接入尚未验证。

1. 使用同事自己明确授权的本机Sunox会话。初次接入只能显式指定一个Chrome配置目录，并用自己已有的作品验证身份；不扫描其他配置、输出凭据、自动启动浏览器或处理验证码。没有授权或登录需要本人时暂停。工具拒绝、安全拒绝或权益不足不能换入口绕过。
2. 先完成所有生成并等扣费结算稳定，再串行下载。工具要求有效Premier、认证actor与clip owner一致、ID/可选完整标题一致，完整账单字段可读；只发Studio准备GET，最长180秒。其他套餐、剪辑编排导出和公开他人作品均未支持。
3. 输出为新的绝对`.wav`路径。已存在文件、目录或链接会在读取认证前拒绝。文件先暂存；同client下载前后积分和普通下载限额/已用/额外余额完全相等才排他交付。任何失败立即停止该条链，核对现有文件与错误，不自动补发、不切普通下载或生成。
4. `saved_unverified`只代表安全保存。使用`tools/check_audio.py`核对页面实际时长、完整解码、字节数及SHA，并登记clip/title/版本/路径/前后额度。已有原件证据仍匹配则复用。失败若没有结束账单快照，不能宣称额度未变。

“免额外消耗”只描述本次有前后账单证据、符合当前权益的成功下载；普通下载剩余0不证明Studio可用，也不代表全账号或永久免费。完整解码、波形、静音技术播放不证明音乐情绪、人声、接缝或听审通过。

CLI无需Ego；需要原生网页菜单备选时，读随包[suno-studio-browser-export](../suno-studio-browser-export/SKILL.md)，它依赖同事另行安装且当前允许的Ego工具。批量制作另读可选[suno-bgm-workflow-portable](../suno-bgm-workflow-portable/SKILL.md)。
