supportedSitePatterns = [
    "*://mangadex.org/chapter/*",
    "*://mangapark.net/title/*/*",
    "*://truyenqq.com.vn/truyen-tranh/*/*/*",
    "*://truyentuan.com/*",
    "*://truyenqqne.com/truyen-tranh/*",
    "*://www.toptruyen.live/truyen-tranh/*/*/*",
    "*://blogtruyen.vn/*/*",
    "*://blogtruyenmoi.com/*/*",
    "*://www.nettruyenmax.com/truyen-tranh/*/*",
    "*://nettruyenhd.com/truyen-tranh/*/*",
];

function downloadChapter(url) {
    chrome.downloads.download({
        // uncomment this for debug
        // url: "http://localhost:8080/download",
        url: "https://manget.fly.dev/download",
        method: "POST",
        headers: [{ name: "content-type", value: "application/json" }],
        body: JSON.stringify({ url }),
    });
}

chrome.runtime.onInstalled.addListener(() => {
    // add context menu on chapter page
    chrome.contextMenus.create({
        id: "chapter_page_download",
        title: "Download chapter",
        contexts: ['page'],
        documentUrlPatterns: supportedSitePatterns,
    });

    // add context menu on chapter link
    chrome.contextMenus.create({
        id: "chapter_link_download",
        title: "Download chapter",
        contexts: ['link'],
        targetUrlPatterns: supportedSitePatterns,
    });
});

// add context menu onclick event
chrome.contextMenus.onClicked.addListener((info, tab) => {
    switch (info.menuItemId) {
        case "chapter_page_download": {
            downloadChapter(info.pageUrl);
            break;
        }
        case "chapter_link_download": {
            downloadChapter(info.linkUrl);
            break;
        }
        default: {
            console.error("Oops, something wrong:\n", info, tab);
        }
    }
});