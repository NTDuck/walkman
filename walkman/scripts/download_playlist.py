import sys
import os
from pathlib import Path
from tqdm import tqdm
from yt_dlp import YoutubeDL
from yt_dlp.utils import DownloadError

def main():
    import argparse

    parser = argparse.ArgumentParser(description="Download audio from a playlist using yt-dlp with progress bars.")
    parser.add_argument("url", help="Playlist URL")
    parser.add_argument("dest", nargs="?", default=".", help="Destination folder (default: current dir)")
    args = parser.parse_args()

    dest = Path(args.dest).resolve()
    dest.mkdir(parents=True, exist_ok=True)

    # Get playlist info without downloading
    ydl_opts_info = {
        "quiet": True,
        "extract_flat": True,
        "force_generic_extractor": False,
    }

    with YoutubeDL(ydl_opts_info) as ydl:
        info = ydl.extract_info(args.url, download=False)
        if "entries" not in info:
            print("Not a playlist URL or couldn't retrieve entries.")
            sys.exit(1)
        entries = info["entries"]
        playlist_title = info.get("title", "Playlist")
        total = len(entries)

    playlist_bar = tqdm(total=total, desc=playlist_title, position=0)

    for idx, entry in enumerate(entries):
        video_url = entry["url"]
        video_title = entry.get("title", f"Track {idx+1}")

        # Placeholder progress bar; will be updated by hook
        audio_bar = tqdm(
            total=100,
            desc=video_title,
            unit="%",
            position=1,
            leave=False,
            bar_format="{desc:.40} | {bar} | {percentage:3.0f}% [{n_fmt}/{total_fmt}]"
        )

        def progress_hook(d):
            if d["status"] == "downloading":
                downloaded = d.get("downloaded_bytes", 0)
                total_bytes = d.get("total_bytes", d.get("total_bytes_estimate", 1))
                speed = d.get("speed", 0)

                if total_bytes > 0:
                    audio_bar.total = total_bytes
                    audio_bar.n = downloaded
                    audio_bar.set_description(f'{d.get("filename", video_title)[:40]}')
                    audio_bar.set_postfix_str(f'{downloaded//1024//1024}MB/{total_bytes//1024//1024}MB')
                    audio_bar.refresh()

            elif d["status"] == "finished":
                audio_bar.n = audio_bar.total
                audio_bar.refresh()
                audio_bar.close()

        ydl_opts_download = {
            "quiet": True,
            "outtmpl": str(dest / "%(title)s.%(ext)s"),
            "format": "bestaudio/best",
            "progress_hooks": [progress_hook],
            "postprocessors": [{
                "key": "FFmpegExtractAudio",
                "preferredcodec": "mp3",
                "preferredquality": "192",
            }]
        }

        try:
            with YoutubeDL(ydl_opts_download) as ydl:
                ydl.download([video_url])
        except DownloadError as e:
            print(f"Failed to download {video_title}: {e}")

        playlist_bar.update(1)

    playlist_bar.close()


if __name__ == "__main__":
    main()
