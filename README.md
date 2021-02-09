# tppocr3

Teseract 4 OCR of Pokemon dialog text on streaming video (version 3).

This project contains experimental, work-in-progress code for running OCR on live streams such as TwitchPlaysPokemon. For tessdata for Tesseract 3 or background information, please see [tppocr](https://github.com/chfoo/tppocr).

## tessdata

TODO: fine-tuned tessdata for Pokemon gen 1, 2, and 3 will go here.

## Software suite

The software suite requires:

* Rust 2018 edition
* Ubuntu 20.04

Dependencies:

        sudo apt install libavcodec-dev libavfilter-dev libavformat-dev libtesseract-dev libtesseract4 libvncserver-dev libvncserver1

Once you install Rust, the Rust versions can be manged with `rustup` command.

Rust programs are managed using the `cargo` command:

1. `cargo build --release`
2. `cargo run --release`

Programs:

1. `stream_dumper`: Decodes each stream frame using ffmpeg's libav libraries and puts it into shared memory.
2. `vnc_server`: Shows a debug image of image detection and recognition in real-time.
3. `tppocr`: Process the results of Tesseract recognition and outputs text in a structured manner.

TODO: more work
