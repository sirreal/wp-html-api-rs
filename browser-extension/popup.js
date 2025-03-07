document.getElementById("analyze").addEventListener("click", () => {
	// Send message directly to service worker
	chrome.runtime.sendMessage({ action: "analyzeTokens" }).catch((error) => {
		console.error("Error sending message to service worker:", error);
	});
});
