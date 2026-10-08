#!/usr/bin/env python3
"""核对已通过 Suno 页面正常下载的原生音频；不登录、不请求网络、不转换文件。"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

VERSION = "1.0.0"


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(chunk)
    return checksum.hexdigest()


def inspect_audio(path, expected, tolerance):
    """同时核对容器时长与实际完整解码，避免把短占位文件当成完整作品。"""
    probe_cmd = ["ffprobe", "-v", "error", "-protocol_whitelist", "file,pipe",
                 "-show_format", "-show_streams", "-of", "json", str(path)]
    probe = subprocess.run(probe_cmd, capture_output=True, text=True, check=True)
    metadata = json.loads(probe.stdout)
    streams = [s for s in metadata["streams"] if s.get("codec_type") == "audio"]
    if len(streams) != 1:
        raise ValueError("需要一条音频流，不能用视频或多轨文件代替单首原生音频")
    stream = streams[0]
    channels, rate = int(stream["channels"]), int(stream["sample_rate"])
    duration = float(metadata["format"]["duration"])
    if channels <= 0 or rate <= 0 or not math.isfinite(duration):
        raise ValueError("音频参数或时长无效")
    if abs(duration - expected) > tolerance:
        raise ValueError(f"文件时长 {duration:.3f} 秒，与页面 {expected:g} 秒不符")
    decode_cmd = ["ffmpeg", "-v", "error", "-xerror", "-err_detect", "explode",
                  "-protocol_whitelist", "file,pipe", "-i", str(path), "-map", "0:a:0",
                  "-vn", "-sn", "-dn", "-f", "s16le", "-acodec", "pcm_s16le", "pipe:1"]
    # PCM 只经过管道计数，不保存第二种格式；原件始终保持不变。
    with tempfile.TemporaryFile() as errors:
        process = subprocess.Popen(decode_cmd, stdout=subprocess.PIPE, stderr=errors)
        decoded_bytes = 0
        with process.stdout as output:
            for chunk in iter(lambda: output.read(1024 * 1024), b""):
                decoded_bytes += len(chunk)
        code = process.wait()
        errors.seek(0)
        error_text = errors.read().decode("utf-8", errors="replace").strip()
    if code != 0 or error_text:
        raise ValueError(f"完整解码失败：{error_text or code}")
    decoded_duration = decoded_bytes / (2 * channels * rate)
    if decoded_bytes == 0 or abs(decoded_duration - expected) > tolerance:
        raise ValueError(f"实际解码 {decoded_duration:.3f} 秒，与页面 {expected:g} 秒不符")
    return {"status": "verified", "path": str(path), "bytes": path.stat().st_size,
            "sha256": digest(path), "codec": stream["codec_name"], "channels": channels,
            "sample_rate": rate, "container_seconds": duration,
            "decoded_seconds": decoded_duration, "expected_seconds": expected,
            "tolerance_seconds": tolerance, "full_decode": True}


def archive(source, destination, source_hash):
    """先完整复制再排他落盘；同内容复用，不覆盖同名不同内容或半成品。"""
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        if destination.is_file() and digest(destination) == source_hash:
            return "reused"
        raise ValueError("目标同名文件内容不同，已保留原件，请换文件名")
    staged = None
    try:
        with tempfile.NamedTemporaryFile(dir=destination.parent, prefix=".suno-copy-", delete=False) as output:
            staged = Path(output.name)
            with source.open("rb") as original:
                shutil.copyfileobj(original, output)
            output.flush()
            os.fsync(output.fileno())
        if digest(staged) != source_hash:
            raise ValueError("复制完整性不符，未归档")
        # link 的目标创建是排他的；同时运行或目标刚被创建也不会覆盖。
        try:
            os.link(staged, destination)
        except FileExistsError:
            if destination.is_file() and digest(destination) == source_hash:
                return "reused"
            raise ValueError("目标刚被其他操作创建，未覆盖") from None
        return "copied"
    finally:
        if staged is not None:
            staged.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(
        description="Suno 页面原生下载后的完整性检查与安全归档（不登录、不联网、不扣额度）。",
        epilog="已实测入口：歌曲 ··· → Edit → Open in Studio → Single-track / Use the full mix → 右键波形 → Download .WAV。保留原生WAV，不另转格式；账号权益以当时页面为准。")
    parser.add_argument("file", type=Path, help="已下载的原生音频路径")
    parser.add_argument("--expected-seconds", type=float, required=True, help="Suno 页面显示的完整曲目秒数")
    parser.add_argument("--tolerance-seconds", type=float, default=1.5, help="容许页面整数显示误差，默认1.5秒")
    parser.add_argument("--archive-to", type=Path, help="可选：校验通过后原样复制到此完整文件名，不覆盖已有文件")
    parser.add_argument("--clip-id", help="可选：在结果中登记已核对的 Suno 曲目ID")
    parser.add_argument("--version", action="version", version=VERSION)
    args = parser.parse_args()
    try:
        if not math.isfinite(args.expected_seconds) or args.expected_seconds <= 0:
            raise ValueError("页面时长必须是正数")
        if not math.isfinite(args.tolerance_seconds) or not 0 <= args.tolerance_seconds <= 3:
            raise ValueError("误差须在0–3秒以内")
        for program in ("ffprobe", "ffmpeg"):
            if shutil.which(program) is None:
                raise ValueError(f"缺少本地检查程序 {program}")
        source = args.file.expanduser().resolve(strict=True)
        if not source.is_file():
            raise ValueError("输入不是文件")
        result = inspect_audio(source, args.expected_seconds, args.tolerance_seconds)
        result.update({"tool_version": VERSION, "clip_id": args.clip_id,
                       "scope": "文件完整性；不证明听感、来源身份或账号额度"})
        if args.archive_to:
            destination = args.archive_to.expanduser().absolute()
            if destination.suffix.lower() != source.suffix.lower():
                raise ValueError("归档须保留原生文件扩展名，不转换格式")
            result["archive"] = {"path": str(destination), "status": archive(source, destination, result["sha256"])}
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(json.dumps({"tool_version": VERSION, "status": "failed", "reason": str(error)}, ensure_ascii=False), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
