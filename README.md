# Suno下载技能包1.0.0

包含3个可发现skill：`suno-native-download`1.0.0（下载主入口）、`suno-studio-browser-export`1.0.0（可选网页助手）、`suno-bgm-workflow-portable`1.1.1+portable.1（可选完整制作记录方法）。包含下载独立便携二进制0.4.0-suno-studio.2、完整固定源码/Cargo.lock、MIT与依赖许可证、音频检查器；不含原账号/音频/认证或构建缓存。

先校验整个zip外部SHA，再解压。包内`SHA256SUMS`列每个文件，`verify-package.py`只读检查完整性/路径安全，不联网、不认证。运行Python3.9+：

```sh
python3 verify-package.py
python3 install.py --dest '/absolute/isolated/skills' --dry-run
python3 install.py --dest '/absolute/your/codex/skills'
```

安装必须明确新目标目录；3个skill任何同名已存在均停止，不覆盖。安装只是复制技能及工具，随主skill保留依赖许可证，不导入认证、不自动创建项目、不启动浏览器或下载。重开Codex/刷新技能列表后使用`$suno-native-download`；工具版本也可直接查看。本机原skill不受影响。

预编译工具仅macOS Apple Silicon；Intel Mac、Windows和Linux没有已验二进制，不能称开箱可用。源码构建需Rust1.88+及公开Cargo依赖，默认离线；构建缓存未打包。Python/ffmpeg/ffprobe及可选Ego需由同事环境提供，安装器不会暗中安装或扩权限。macOS出现来源/签名拦截，先核对SHA/来源，由用户按公司安全要求正常处理，不自动删除隔离标记或关闭系统保护。

安装后读取主skill的[调用说明](skills/suno-native-download/references/usage.md)，由同事明确允许工具接入**自己的一个Chrome配置目录**，用自有作品核对身份和Premier，再保存必要本机会话。只下载自己的已有曲；凭据不可写进项目或发给同事。本包未实测接入同事账号，不承诺全账号免积分；原生WAV是MP3不可用时的单格式备用。批量生成与下载不能并发。

许可见`LICENSE`、`skills/suno-native-download/tools/LICENSE.sunox`和主skill内的`tools/licenses/`；固定来源与修改范围见`provenance.json`。原Sunox通用模块保留是源码编译依赖，便携main不暴露其生成/验证码/普通下载入口。`verification-summary.json`区分本次离线测试和匿名历史真实下载事实，安装dry-run不是账号验收。
