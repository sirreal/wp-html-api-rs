document.getElementById("analyzeClean").addEventListener(
	"click",
	() => {
		chrome.runtime.sendMessage({ action: "analyzeClean" });
	},
	{ passive: true },
);
document.getElementById("analyzeDom").addEventListener(
	"click",
	() => {
		chrome.runtime.sendMessage({ action: "analyzeDom" });
	},
	{ passive: true },
);
