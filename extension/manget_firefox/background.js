SUPPORTED_SITE_PATTERNS = [
    "*://mangadex.org/chapter/*",
    "*://mangapark.net/title/*/*",
    "*://truyenqq.com.vn/truyen-tranh/*/*/*",
    "*://truyentuan.com/*",
    "*://truyenqqne.com/truyen-tranh/*",
    "*://www.toptruyen.live/truyen-tranh/*/*/*",
    "*://blogtruyen.vn/*/*",
    "*://blogtruyenmoi.com/*/*",
    "*://m.blogtruyenmoi.com/*/*",
    "*://www.nettruyenmax.com/truyen-tranh/*/*",
    "*://nettruyenhd.com/truyen-tranh/*/*",
    "*://www.nettruyenus.com/truyen-tranh/*/*/*",
    "*://nettruyenco.vn/truyen-tranh/*/*/*"
];

// BASE_URL = "http://localhost:8080"
BASE_URL = "https://manget.fly.dev"

async function downloadChapter(url) {
    let res = await fetch(BASE_URL + "/get_chapter_info", {
        method: "POST",
        headers: {
            "content-type": "application/json",
        },
        body: JSON.stringify({ url: url })
    },
    );
    if (res.ok) {
        let body = await res.json();
        await browser.downloads.download({
            url: BASE_URL + "/download",
            method: "POST",
            headers: [{ name: "content-type", value: "application/json" }],
            body: JSON.stringify({ url }),
            filename: body.chapter_name + ".cbz",
        });
    } else {
        console.error(`Response status: ${res.statusText}. Response body: ${await res.text()}`)
    }
}

// add context menu on chapter page
browser.contextMenus.create({
    id: "chapter_page_download",
    title: "Download chapter",
    contexts: ['page'],
    documentUrlPatterns: SUPPORTED_SITE_PATTERNS,
});

// add context menu on chapter link
browser.contextMenus.create({
    id: "chapter_link_download",
    title: "Download chapter",
    contexts: ['link'],
    targetUrlPatterns: SUPPORTED_SITE_PATTERNS,
});

// add context menu onclick event
browser.contextMenus.onClicked.addListener(async (info, tab) => {
    switch (info.menuItemId) {
        case "chapter_page_download": {
            await downloadChapter(info.pageUrl);
            break;
        }
        case "chapter_link_download": {
            await downloadChapter(info.linkUrl);
            break;
        }
        default: {
            console.error("Oops, something wrong:\n", info, tab);
        }
    }
});