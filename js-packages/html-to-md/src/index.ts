import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { HtmlToMarkdown } from "./html-to-markdown.js";

// Create an MCP server
const server = new McpServer({
	name: "HTML to Markdown",
	version: "0.0.1",
});

// Add HTML to Markdown conversion tool
server.tool(
	"convertHtmlToMarkdown",
	{
		html: z.string().describe("The HTML content to convert to Markdown"),
		baseUrl: z
			.string()
			.optional()
			.describe("Optional base URL for resolving relative links"),
		width: z
			.number()
			.optional()
			.default(80)
			.describe("Optional maximum line width"),
	},
	async ({ html, baseUrl = "", width = 80 }) => {
		try {
			const markdown = HtmlToMarkdown.convert(html, baseUrl, width);

			return {
				content: [
					{
						type: "text",
						text: markdown,
					},
				],
			};
		} catch (error) {
			return {
				content: [
					{
						type: "text",
						text: `Error converting HTML to Markdown: ${
							error instanceof Error ? error.message : String(error)
						}`,
					},
				],
			};
		}
	},
);

// Add fetch and convert tool - fetching happens directly in this tool
server.tool(
	"fetchAndConvertToMarkdown",
	{
		url: z
			.string()
			.url()
			.describe("URL of the webpage to fetch and convert to Markdown"),
		width: z
			.number()
			.optional()
			.default(80)
			.describe("Optional maximum line width"),
	},
	async ({ url, width = 80 }) => {
		try {
			// Fetch the HTML content
			const response = await fetch(url);

			if (!response.ok) {
				throw new Error(
					`Failed to fetch URL: ${response.status} ${response.statusText}`,
				);
			}

			const html = await response.text();
			const baseUrl = new URL(url).origin;

			// Convert the HTML to Markdown
			const markdown = HtmlToMarkdown.convert(html, baseUrl, width);

			return {
				content: [
					{
						type: "text",
						text: markdown,
					},
				],
			};
		} catch (error) {
			return {
				content: [
					{
						type: "text",
						text: `Error fetching or converting content from ${url}: ${
							error instanceof Error ? error.message : String(error)
						}`,
					},
				],
			};
		}
	},
);

// Start receiving messages on stdin and sending messages on stdout
const transport = new StdioServerTransport();

// Start the server
async function main() {
	console.error("Starting HTML to Markdown MCP server...");
	await server.connect(transport);
}

await main().catch((err) => {
	console.error("Failed to start server:", err);
        throw err
});
