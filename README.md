![rust build](https://github.com/ndtoan96/manget/actions/workflows/rust.yml/badge.svg)

This project is for personal use, but I think it is nice to make it public so other people with the same need can use it.

Manget is a collection of tools written in Rust which help download manga from all over the internet and zip it to a `cbz` format so it can be read by an e-reader. Currently supported sites are:
- [mangapark](https://mangapark.net/)
- [mangadex](https://mangadex.org/)
- [mangapark](https://mangapark.net/)
- [nettruyen](https://www.nettruyenmax.com/)
- [toptruyen](https://www.toptruyenne.com/)
- [truyenqq](https://truyenqq.com.vn/)
- [truyentuan](https://truyentuan.com/)

This project includes:
- **manget**: the core library
- **manget_cli**: a cli tool to download manga to local PC
- **manget_server**: a server that provides an api to download manga. This is typically used in tandem with a custom made browser extension.

You're most likely interested in the `manget_cli` tool, which can be downloaded from the [release page](https://github.com/ndtoan96/manget/releases). This tool has 2 modes: download one chapter and download a list of chapters:
- Download one chapter: `manget_cli <url>`
- Download list of chapters: `manget_cli -f <file>`. Where `<file>` is a text file contains list of chapter urls.

Run `manget_cli -h` for more detail.
