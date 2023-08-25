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
- **manget_server**: a server that provides an api to download manga. This is typically used in tandem with an custom made browser extension.
