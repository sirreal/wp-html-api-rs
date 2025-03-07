// Import the WASM module
import initWasm, { WP_HTML_Processor } from "./wp_html_api_wasm.js";

let fmt;

// Handle service worker installation
self.addEventListener("install", async (event) => {
	await initWasm();

	fmt = Intl.NumberFormat("en-US");
	self.skipWaiting();
});

// Process message from popup
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
	if (message.action === "analyzeTokens") {
		handleAnalyzeTokens();
	}
});

// Handle analyze tokens request
function handleAnalyzeTokens() {
	chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
		if (tabs && tabs.length > 0) {
			analyzeTokens(tabs[0].id);
		} else {
			console.error("No active tab found");
		}
	});
}

// Analyze HTML tokens
async function analyzeTokens(tabId) {
	try {
		// Get the HTML content of the page using content script
		chrome.scripting.executeScript(
			{
				target: { tabId },
				function: async () => {
					const resp = await fetch(document.location.href);
					return await resp.text();
				},
			},
			([{ result: htmlText, error }]) => {
				if (error) {
					console.error(error);
					return;
				}
				processHTML(htmlText, tabId);
			},
		);
	} catch (error) {
		console.error(error);
	}
}

// Process the HTML with the WASM module
function processHTML(html, tabId) {
	const byteLength = new TextEncoder().encode(html).byteLength;
	// Create the HTML processor
	const processor = WP_HTML_Processor.create_full_parser(html);

	if (!processor) {
		console.error("Failed to create HTML processor");
		return;
	}

	// Count token types
	const tokenCounts = new Map();
	let totalTokens = 0;

	// Process all tokens
	const start = performance.now();
	while (processor.next_token()) {
		const tokenType = processor.get_token_type();
		if (tokenType) {
			let c = tokenCounts.get(tokenType) ?? 0;
			tokenCounts.set(tokenType, c + 1);
			totalTokens++;
		}
	}
	const done = performance.now();
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
