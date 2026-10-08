# 安装与调用

包版本1.0.0；独立skill1.0.0；工具0.4.0-suno-studio.2（便携修订）。解压包根`README.md`给出校验和安装步骤。安装后把下列`SKILL_DIR`换成此skill的真实安装目录；不要把占位符当成命令执行。

## 依赖与接入

- 预编译程序：macOS Apple Silicon（arm64）。CLI本身不依赖Ego或Node；动态系统库列表在包根`platform.json`。
- 完整音频检查：Python3（3.9+）、系统可用的`ffmpeg`和`ffprobe`。这些独立工具未打包，不自动安装；工具先安全保存，检查器缺失时不得宣称完整性通过。
- 初次会话导入：用户在自己明确指定的Chrome配置中正常登录Suno，并明确允许此下载工具在本机处理/保存必要会话。可能需要本人允许系统钥匙串正常访问；拒绝/解锁/2FA/验证码交本人。不复制他人的会话，不传cookie/JWT参数或打印认证文件。
- 工具使用Sunox本机配置位置：设置了`XDG_CONFIG_HOME`时是其下`sunox/auth.json`，否则使用当前用户平台的Sunox配置目录。已有不同或身份未知会话不覆盖。必要会话仅本机保存，不放进项目、分享包或Git。

一次接入（需要当前用户明确授权，**不是安装步骤**）：

```sh
"SKILL_DIR/tools/sunox-studio-download.sh" 'https://suno.com/song/OWNED_CLIP_UUID' \
  --connect-chrome-profile '/absolute/path/to/your/Chrome/profile' \
  --expect-title '自己作品的完整标题'
```

只处理这一目录，不扫描、回退其他profile或浏览器。用正常浏览器的配置位置确认实际目录，不根据同事或历史账号推断。返回`connected`及`actor_owner_matches:true`才说明接入已通过当前归属与Premier验证。

每首下载：

```sh
"SKILL_DIR/tools/sunox-studio-download.sh" 'https://suno.com/song/OWNED_CLIP_UUID' \
  '/absolute/new-output.wav' --expect-title '自己作品的完整标题'
python3 "SKILL_DIR/tools/check_audio.py" '/absolute/new-output.wav' --expected-seconds 90
```

`90`只是示例，必须换为该首实际页面时长；目标创作时长不等于实际时长。先用`--help`核对检查器参数；它只读原文件，解码PCM经过管道计数，不保存第二种格式。下载JSON不能含signed URL或认证字段；失败保留脱敏错误，不执行无限重试。浏览器或其他任务不要在下载期间并发生成，正常生成扣费会触发账单保护。

帮助/版本不读会话、不联网：

```sh
"SKILL_DIR/tools/sunox-studio-download.sh" --help
"SKILL_DIR/tools/sunox-studio-download.sh" --version
```

## 复现与适用范围

`tools/source/`是完整固定源码、Cargo.lock、编译所需资源及MIT许可证；`tools/build.sh`使用本机Rust1.88+构建，默认离线，未打包Cargo依赖缓存。缺依赖时停止；如同事允许正常下载公开构建依赖，可明确传`--online`。没有认证读取或账号请求。源码包含上游构建所需通用模块，但本包main无条件只进入Studio专用入口，即使改二进制文件名也不能启动生成/普通下载/验证码流程。

当前真实历史验证：原0.4.0-suno-studio.1在一个Premier账号完成3首4次、重复字节相同，后续46首补齐及多个生成批次原生导出，最近单类2首同client账单均不变。分享包不含真实账号、曲名、clip、音频、登录态或原始私有日志。便携修订只做本地模拟合同/构建/安装测试，不把历史原版成功称作便携版同事账号成功。服务端网页接口可能变化，失败按实际证据停止。

来源：[官方下载权益](https://help.suno.com/en/articles/13876865)、[Sunox MIT项目](https://github.com/ctykwz/sunox)、[固定社区Studio路由代码](https://github.com/paperfoot/suno-cli/blob/6d28c67a733b32183a80d81306bf0013f46b00b7/src/api/downloads.rs)。这不是官方SDK或保证下载权益的声明。
