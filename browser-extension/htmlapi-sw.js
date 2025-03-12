// Import the WASM module
import initWasm, { WP_HTML_Processor } from "./wp_html_api_wasm.js";


// Handle service worker installation
self.addEventListener("install", async (event) => {
	event.waitUntil( initWasm() );
});


const fmt = Intl.NumberFormat("en-US");

// Process message from popup
chrome.runtime.onMessage.addListener(async (message, sender, sendResponse) => {
	const tabId = await new Promise((resolve, reject) => {
		chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
			if (tabs && tabs.length > 0) {
				resolve(tabs[0].id);
			} else {
				reject();
			}
		});
	});

	let html = null;
	switch (message.action) {
		case "analyzeClean":
			html = await new Promise((resolve, reject) => {
				chrome.scripting.executeScript(
					{
						target: { tabId },
						function: async () => {
							const resp = await fetch(document.location.href);
							return await resp.text();
						},
					},
					([{ result, error }]) => {
						if (error) {
							reject(error);
						} else {
							resolve(result);
						}
					},
				);
			});
			break;

		case "analyzeDom":
			html = await new Promise((resolve, reject) => {
				chrome.scripting.executeScript(
					{
						target: { tabId },
						function: async () => {
							return document.documentElement.outerHTML;
						},
					},
					([{ result, error }]) => {
						if (error) {
							reject(error);
						} else {
							resolve(result);
						}
					},
				);
			});
			break;
	}

	if (html != null) {
		processHTML(html, tabId);
	}
});

// Process the HTML with the WASM module
function processHTML(html, tabId) {
	const byteLength = new TextEncoder().encode(html).byteLength;

	// Count token types
	const tokenCounts = new Map();
	let totalTokens = 0;
	let start, done;

	// Create the HTML processor
	const processor = WP_HTML_Processor.create_full_parser(html);
	try {
		if (!processor) {
			console.error("Failed to create HTML processor");
			return;
		}

		// Process all tokens
		start = performance.now();
		while (processor.next_token()) {
			const tokenType = processor.get_token_type();
			if (tokenType) {
				let c = tokenCounts.get(tokenType) ?? 0;
				tokenCounts.set(tokenType, c + 1);
				totalTokens++;
			}
		}
		done = performance.now();
	} finally {
		processor.free();
	}
	const mbps = fmt.format(byteLength / 1e6 / ((done - start) / 1e3));
	const ms = fmt.format(done - start);

	// Format the results
	const results = Array.from(tokenCounts.entries())
		.map(([type, count]) => `- ${type}: ${fmt.format(count)}`)
		.join("\n");

	// Show the results in an alert on the page
	chrome.scripting.executeScript({
		target: { tabId },
		function: (tokenResults, totalTokens, ms, mbps) => {
			alert(
				`HTML Token Counts:\n${tokenResults}\nTotal Tokens: ${totalTokens}\nTook ${ms}ms (${mbps}MB/s)`,
			);
		},
		args: [results, fmt.format(totalTokens), ms, mbps],
	});
}
