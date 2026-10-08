---
name: suno-studio-browser-export
description: Use Suno's visible Studio original-clip WAV menu in an authorized Ego session when a browser export is appropriate; do not generate or transfer credentials.
metadata:
  version: "1.0.0"
  source_helper_version: "1.0.0"
  source_ego_skill_version: "2.0.1"
---

# Studio网页导出备选

这是分享专用精简入口，非原Ego技能副本。需要同事自己安装并授权当前Ego工具；本包不含Ego应用、运行时、登录态或其整套手册。先读当前环境的官方/已安装Ego技能与API，工具权限高于此说明。

由唯一当前浏览器控制者，核对自己的账号/原曲/有效Studio权益和前后额度。在正常页面用歌曲菜单→Edit→Open in Studio→Single-track / Use the full mix，等真实波形；右键原曲clip→Download .WAV。原生MP3不可用时只这一份WAV。不能把历史选择器、页面标签或坐标当当前事实。

可选`sh scripts/export_studio.sh --help`查看原子菜单导出助手。它依赖`ego-browser nodejs`和对应API；先确认当前唯一canvas和原曲单轨身份，页面body含标题不能独立证明来源。菜单、arm下载、点击及保存要在同一次Ego调用内完成。输出必须新的绝对.wav路径，现有文件保留。它不代做权益/账单验证，成功仅`saved_unverified`。

原生网页菜单曾实际保存完整音频；此助手**未完成独立真实成功验收**，只提供源码及离线语法检查，不能冒称已验稳定工具。优先使用[suno-native-download](../suno-native-download/SKILL.md)的guarded CLI。engine初始化不完、下载事件超时或工具拒绝，保留失败并停止；不盲点、不换认证入口、不注入token/solver。完成后用该skill的check_audio.py校验完整性，按Ego生命周期正常交还用户，保留用户页面。
