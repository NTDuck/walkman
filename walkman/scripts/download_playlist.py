from argparse import ArgumentParser
import logging
import math
import sys
from pathlib import Path
from typing import Any
from tqdm import tqdm
from yt_dlp import YoutubeDL
from yt_dlp.utils import DownloadError


def main():
    parser = ArgumentParser()
    parser.add_argument("-i", "--upstream-uri")
    parser.add_argument("-o", "--downstream-uri", nargs="?", default=".")

    args = parser.parse_args()
    upstream_uri = args.upstream_uri
    downstream_uri = args.downstream_uri

    downstream_uri = Path(downstream_uri).resolve()
    downstream_uri.mkdir(parents=True, exist_ok=True)

    ydl_opts = {
        "quiet": True,
        "extract_flat": True,
        "force_generic_extractor": False,
    }

    try:
        with YoutubeDL(ydl_opts) as ydl:
            info = ydl.extract_info(url=upstream_uri, download=False)

            if "entries" not in info:
                logging.error("Invalid URI")
                sys.exit(1)

            playlist_title = info["title"]
            videos = info["entries"]
            video_count = len(videos)
        
    except DownloadError as err:
        logging.error(err)

    video_progress_bar = tqdm(
        initial=math.nan,
        total=math.nan,
    )

    playlist_progress_bar = tqdm(
        desc=playlist_title,
        initial=0,
        total=video_count,
    )

    for idx, video in enumerate(videos):
        video_title = video["title"]
        video_progress_bar.set_description(video_title)
        video_progress_bar.initial = 0

        def progress_hook(d: dict[str, Any]):
            status = d["status"]

            if status == "downloading":
                downloaded_bytes = d["downloaded_bytes"]
                total_bytes = d["total_bytes"]

                video_progress_bar.n = downloaded_bytes
                video_progress_bar.total = total_bytes

            elif status == "error":
                pass

            elif status == "finished":
                video_progress_bar.clear()
                playlist_progress_bar.update(1)

        ydl_opts = {
            "quiet": True,
            "no_warnings": True,
            "outtmpl": str(downstream_uri / "%(title)s.%(ext)s"),
            "format": "bestaudio/best",
            "postprocessors": [{
                "key": "FFmpegExtractAudio",
                "preferredcodec": "mp3",
                "preferredquality": "128",  # medium quality
            }],
            "logger": None,
        }

        try:
            with YoutubeDL(ydl_opts) as ydl:
                ydl.add_progress_hook(progress_hook)
                ydl.download([video["url"]])
        
        except DownloadError as err:
            logging.error(err)

    playlist_progress_bar.close()


if __name__ == "__main__":
    main()
