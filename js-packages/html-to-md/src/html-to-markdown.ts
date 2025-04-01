import fs from 'node:fs/promises';
import { initSync, WP_HTML_Processor, WP_HTML_Tag_Processor } from "../pkg-node/wp_html_api_wasm.js";

const wasmBuffer = await fs.readFile(
  new URL('../pkg-node/wp_html_api_wasm_bg.wasm',import.meta.url)
);
initSync( {module:wasmBuffer});

/**
 * Class providing a generic HTML-to-Markdown transformation.
 *
 * This class is powered by the HTML API and will properly
 * understand HTML parsing semantics, meaning that it's safe
 * to pass in HTML with "unbalanced" tags and other oddities.
 *
 * Markdown syntax is based off of the CommonMark spec with
 * (planned) extensions for GitHub Flavored Markdown (GFM).
 */
export class HtmlToMarkdown {
	// Used to ensure that formatting boundaries apply as syntax
	private static readonly SEP = "\u2063"; // INVISIBLE SEPARATOR
	
	// Reusable text segmenters for word and grapheme processing
	private static readonly WORD_SEGMENTER = new Intl.Segmenter(undefined, { granularity: 'word' });
	private static readonly GRAPHEME_SEGMENTER = new Intl.Segmenter(undefined, { granularity: 'grapheme' });

	/**
	 * Known programming languages for code blocks
	 */
	private static readonly KNOWN_LANGUAGES = [
		"apl",
		"asm",
		"assembly",
		"bash",
		"c",
		"c#",
		"c++",
		"clojure",
		"cobol",
		"cpp",
		"csharp",
		"css",
		"d",
		"dart",
		"elixir",
		"elm",
		"erlang",
		"f#",
		"fish",
		"fortran",
		"fsharp",
		"go",
		"groovy",
		"guile",
		"haskell",
		"html",
		"java",
		"javascript",
		"js",
		"julia",
		"kotlin",
		"less",
		"lisp",
		"lua",
		"matlab",
		"objectivec",
		"objective-c",
		"ocaml",
		"perl",
		"php",
		"powershell",
		"python",
		"python2",
		"python3",
		"r",
		"racket",
		"raku",
		"ruby",
		"rust",
		"sass",
		"scala",
		"scheme",
		"sgml",
		"sh",
		"shell",
		"sql",
		"swift",
		"typescript",
		"ts",
		"vba",
		"xml",
		"zsh",
	];

	/**
	 * Converts a given HTML document into a corresponding Markdown document.
	 *
	 * @param html HTML to convert
	 * @param baseUrl Base URL for the page, if provided, otherwise inferred from the HTML
	 * @param width Approximate max line length
	 * @returns Markdown representation of input HTML
	 */
	public static convert(html: string, baseUrl = "", width = 80): string {
		const preprocessedHtml = HtmlToMarkdown.preprocessInputStream(html);

		// Create HTML processor to scan through the input HTML document
		const scanner = WP_HTML_Processor.create_full_parser(preprocessedHtml);
		if (!scanner) {
			return HtmlToMarkdown.fallback(preprocessedHtml);
		}

		// Output buffer containing fully-processed Markdown text
		let md = "";

		/**
		 * Stores type of every open un/ordered list and its counter.
		 * Used when flushing the line buffer into the output markdown.
		 *
		 * Example: For <ol><li><li><ul><li>HERE</ul></ul>
		 * array would be [ 'decimal', 2, '-', 1 ]
		 */
		let olCounts: Array<string | number> = [];

		// Buffers the current line before performing indentation, prefixing, and wrapping
		let line = "";

		// Stores attributes from the last-parsed tag
		let lastAttrs: Record<string, string> = {};

		// Temporarily traps the line buffer while processing links
		let linkSwap = "";

		// Track nested formatting tags
		let emDepth = 0;
		let strongDepth = 0;

		// Helper function to flush the current line to the output
		const flushLine = () => {
			let firstPrefix = "";
			let linePrefix = "";
			let inPre = false;
			let noNewlines = false;
			let listDepth = 0;

			// Get breadcrumbs (skip HTML and BODY elements)
			const breadcrumbs = scanner.get_breadcrumbs().slice(2);

			// Block-level elements create line prefixes. These go in order.
			for (const tag of breadcrumbs) {
				switch (tag) {
					case "BLOCKQUOTE":
						firstPrefix += "> ";
						linePrefix += "> ";
						break;

					case "CODE":
						if (inPre) {
							firstPrefix += "    ";
							linePrefix += "    ";
						}
						break;

					case "LI":
						if (listDepth === 0) {
							break;
						}

						const ld = listDepth - 1;
						const listType = olCounts[ld * 2] as string;
						const count = olCounts[ld * 2 + 1] as number;

						const marker = HtmlToMarkdown.listMarker(
							listType,
							listType === "-" ? listDepth : count,
						);
						const indent = " ".repeat(HtmlToMarkdown.graphemeLength(marker));

						if (listDepth !== olCounts.length / 2) {
							firstPrefix += `${indent} `;
						} else {
							firstPrefix += `${marker} `;
						}
						linePrefix += `${indent} `;
						break;

					case "PRE":
						inPre = true;
						break;

					case "H1":
					case "H2":
					case "H3":
					case "H4":
					case "H5":
					case "H6":
						noNewlines = true;
						break;

					case "OL":
					case "UL":
						listDepth++;
						break;
				}
			}

			if (!inPre) {
				line = line.trim();
			}

			if (noNewlines) {
				md += `${firstPrefix}${line}\n`;
				line = "";
				return;
			}

			const prefix = linePrefix;
			let lineLength = HtmlToMarkdown.graphemeLength(firstPrefix);
			const prefixLength = HtmlToMarkdown.graphemeLength(linePrefix);

			// Split line into words for wrapping
			const words = HtmlToMarkdown.splitWords(line);

			for (let i = 0; i < words.length; i++) {
				const word = words[i];

				// Handle newlines in text
				if (/^\n+$/.test(word)) {
					md += `${word[0]}${prefix}`;
					lineLength = prefixLength;
					continue;
				}

				if (i === 0) {
					md += firstPrefix;
				}

				const wordLength = HtmlToMarkdown.graphemeLength(word);

				// Keep trailing punctuation on the same line
				const isPunctuation = /^[,.?!]+$/.test(word.trim());

				if (wordLength + lineLength > width && !isPunctuation) {
					const trimmedWord = word.trimStart();
					md += `\n${prefix}${trimmedWord}`;
					lineLength =
						prefixLength + HtmlToMarkdown.graphemeLength(trimmedWord);
				} else {
					md += word;
					lineLength += wordLength;
				}
			}

			md += "\n";
			line = "";
		};

		// Helper function to append text to the current line
		const append = (chunk: string) => {
			if (!scanner.get_breadcrumbs().includes("PRE")) {
				chunk = chunk.replace(/[ \t]+\n+/g, "\n");
				chunk = chunk.replace(/[ \t]+/g, " ");
				chunk = chunk.replace(/[\f\n]+/g, "\n");
			}
			line += chunk;
		};

		// Helper function to remember attributes from a tag
		const remember = (attributes: string[]) => {
			lastAttrs = {};

			for (const attr of attributes) {
				let value = scanner.get_attribute(attr);
				if (value === true) {
					value = "";
				}

				if (value !== null && value !== undefined) {
					lastAttrs[attr] = String(value);
				}
			}
		};

		// Main processing loop
		while (scanner.next_token()) {
			const tokenNameBytes = scanner.get_token_name();
			if (!tokenNameBytes) continue;

			const tokenName = new TextDecoder().decode(tokenNameBytes);
			const isCloser = scanner.is_tag_closer();
			const breadcrumbs = scanner.get_breadcrumbs().slice(2); // Chop off HTML and BODY

			switch (tokenName) {
				case "#text":
					const textBytes = scanner.get_modifiable_text();
					const text = textBytes ? new TextDecoder().decode(textBytes) : "";

					if (/^[ ]*[\n]+$/.test(text)) {
						break;
					}

					if (!breadcrumbs.includes("PRE")) {
						append(HtmlToMarkdown.escapeAsciiPunctuation(text));
					} else {
						append(text);
					}
					break;

				case "A":
					if (isCloser) {
						const url = HtmlToMarkdown.toUrl(lastAttrs["href"] || "", baseUrl);
						const escapedUrl = HtmlToMarkdown.escapeAsciiPunctuation(url);
						const linkLabel = line.trim();
						line = linkSwap;

						const title = lastAttrs["title"]
							? ` "${HtmlToMarkdown.escapeAsciiPunctuation(
									lastAttrs["title"],
								)}"`
							: "";

						if (!url) {
							append(linkLabel);
						} else {
							append(`[${linkLabel}](${escapedUrl}${title})`);
						}
					} else {
						remember(["href", "title"]);
						linkSwap = line;
						line = "";
					}
					break;

				case "B":
				case "STRONG":
					strongDepth += isCloser ? -1 : 1;
					if (
						(strongDepth === 1 && !isCloser) ||
						(strongDepth === 0 && isCloser)
					) {
						const leftFlank = isCloser ? "" : HtmlToMarkdown.SEP;
						const rightFlank = isCloser ? HtmlToMarkdown.SEP : "";
						append(`${leftFlank}**${rightFlank}`);
					}
					break;

				case "BASE":
					if (baseUrl) {
						break;
					}

					const href = scanner.get_attribute("href");
					if (typeof href !== "string") {
						break;
					}

					const trimmedHref = String(href).trim();
					if (trimmedHref) {
						baseUrl = HtmlToMarkdown.toUrl(trimmedHref, baseUrl);
					}
					break;

				case "BR":
					if (line.length > 0) {
						append("  ");
					}
					flushLine();
					break;

				case "CODE":
					if (breadcrumbs.includes("PRE")) {
						if (isCloser) {
							flushLine();
							append("```");
							flushLine();
						} else {
							flushLine();
							append("```");
							let lang = "";

							// Try to extract the language from the CSS class names
							const classList = scanner.class_list();

							for (const className of classList) {
								const lowerClassName = className.toLowerCase();

								if (lowerClassName.startsWith("language-")) {
									lang = lowerClassName.substring("language-".length);
									break;
								}

								if (HtmlToMarkdown.KNOWN_LANGUAGES.includes(lowerClassName)) {
									lang = lowerClassName;
									break;
								}
							}

							lang = lang.trim();
							if (!lang || lang.endsWith("`")) {
								lang = "";
							}

							// Look in specific attributes if the language isn't yet inferred
							if (!lang) {
								const langAttrs = [
									"data-lang",
									"data-language",
									"data-codetag",
									"syntax",
									"data-programming-language",
									"type",
								];

								for (const attribute of langAttrs) {
									const dataLang = scanner.get_attribute(attribute);
									if (typeof dataLang === "string") {
										const trimmedLang = dataLang.trim();
										if (HtmlToMarkdown.KNOWN_LANGUAGES.includes(trimmedLang)) {
											lang = trimmedLang;
											break;
										}
									}
								}
							}

							lang = lang.trim();
							if (!lang || lang.endsWith("`")) {
								lang = "";
							}

							if (lang) {
								append(lang);
							}
						}
						flushLine();
					} else {
						append("`");
					}
					break;

				case "H1":
				case "H2":
				case "H3":
				case "H4":
				case "H5":
				case "H6":
					if (isCloser) {
						line = line.trim();
						flushLine();
					} else {
						append("\n");
						flushLine();
						append("#".repeat(parseInt(tokenName.substring(1), 10)) + " ");
					}
					break;

				case "HR":
					flushLine();
					append("***"); // Use '*' to avoid clashes with settext_headings, which use '-'
					flushLine();
					break;

				case "I":
				case "EM":
					emDepth += isCloser ? -1 : 1;
					if ((emDepth === 1 && !isCloser) || (emDepth === 0 && isCloser)) {
						const leftFlank = isCloser ? "" : HtmlToMarkdown.SEP;
						const rightFlank = isCloser ? HtmlToMarkdown.SEP : "";
						append(`${leftFlank}_${rightFlank}`);
					}
					break;

				case "IMG":
					const alt = scanner.get_attribute("alt") || "";
					const src = scanner.get_attribute("src") || "";
					const url = HtmlToMarkdown.toUrl(String(src).trim(), baseUrl);
					const escapedUrl = HtmlToMarkdown.escapeAsciiPunctuation(url);

					let title = scanner.get_attribute("title");
					if (typeof title !== "string") {
						title = "";
					}

					const titleAttr = title
						? ` "${HtmlToMarkdown.escapeAsciiPunctuation(title)}"`
						: "";

					append(`![${alt}](${escapedUrl}${titleAttr})`);
					break;

				case "LI":
					if (isCloser) {
						break;
					}
					flushLine();
					if (olCounts.length > 0) {
						olCounts[olCounts.length - 1] =
							(olCounts[olCounts.length - 1] as number) + 1;
					}
					break;

				case "OL":
					flushLine();
					if (isCloser) {
						olCounts.pop();
						olCounts.pop();
					} else {
						olCounts.push("decimal");
						olCounts.push(0);
					}
					break;

				case "UL":
					flushLine();
					if (isCloser) {
						olCounts.pop();
						olCounts.pop();
					} else {
						olCounts.push("-");
						olCounts.push(0);
					}
					break;

				// Block-elements
				case "BLOCKQUOTE":
				case "P":
					flushLine();
					break;
			}
		}

		// Check for parsing errors
		const lastError = scanner.get_last_error();
		if (lastError && md.length < 50) {
			return HtmlToMarkdown.fallback(html);
		}

		flushLine();
		return md.trim();
	}

	/**
	 * Failure mechanism in case the HTML Processor is unable
	 * to fully parse the given HTML
	 */
	private static fallback(html: string): string {
		// This is the most primitive conversion to avoid throwing an exception
		const output: string[] = [];

		try {
			const processor = new WP_HTML_Tag_Processor(html);

			while (processor.next_token()) {
				const tokenNameBytes = processor.get_token_name();
				if (!tokenNameBytes) continue;

				const tokenName = new TextDecoder().decode(tokenNameBytes);

				if (tokenName === "#text") {
					const textBytes = processor.get_modifiable_text();
					if (textBytes) {
						output.push(new TextDecoder().decode(textBytes));
					}
				}
			}

			return output.join("");
		} catch (e) {
			// Even more basic fallback if tag processor fails
			return html.replace(/<[^>]*>/g, "");
		}
	}

	/**
	 * Follows the HTML preprocess-the-input-stream algorithm.
	 *
	 * @param html Input HTML to preprocess
	 * @returns Preprocessed output HTML
	 */
	private static preprocessInputStream(html: string): string {
		return html.replace(/\r\n|\r/g, "\n");
	}

	/**
	 * Escapes ASCII punctuation characters in plaintext so they won't
	 * be interpreted as Markdown syntax.
	 *
	 * @param plaintext Text to escape
	 * @returns Escaped text
	 */
	private static escapeAsciiPunctuation(plaintext: string): string {
		return plaintext.replace(/[!"#$%&'()*+,-./:;<=>?@[\\\]^_`{|}~]/g, "\\$&");
	}

	/**
	 * Returns a list marker given a list type and count.
	 *
	 * @param listType One of '-', 'decimal'
	 * @param count Position of this list item in a list
	 * @returns List marker as a string
	 */
	private static listMarker(listType: string, count: number): string {
		switch (listType) {
			case "-":
				// Sibling lists should alternate between *, +, and -
				return "*+-"[count % 3];

			case "decimal":
				// The reason for the length limit is that with 10 digits we start
				// seeing integer overflows in some browsers.
				return `${Math.max(1, Math.min(count, 999999999))}.`;
		}

		// This should not be reachable
		return "";
	}

	/**
	 * Normalizes URLs and joins a base URL to relative paths, if provided.
	 *
	 * @param href Absolute or relative HREF value from an A element
	 * @param baseUrl Base URL for the HTML, if available
	 * @returns Absolute and resolved URL associated with link
	 */
	private static toUrl(href: string, baseUrl = ""): string {
		if (!/^(?:https?|mailto):\/\//.test(href)) {
			const base = !baseUrl ? "/" : baseUrl;
			return `${base}${href}`;
		}

		return href;
	}

	/**
	 * Split text into words for line wrapping using the reusable word segmenter
	 *
	 * @param text Text to split
	 * @returns Array of words
	 */
	private static splitWords(text: string): string[] {
		const segments = this.WORD_SEGMENTER.segment(text);
		const words: string[] = [];
		
		for (const segment of segments) {
			words.push(segment.segment);
		}
		
		return words.filter(Boolean);
	}

	/**
	 * Get the length of a string in grapheme clusters
	 *
	 * @param str String to measure
	 * @returns Length in grapheme clusters
	 */
	private static graphemeLength(str: string): number {
		const segments = this.GRAPHEME_SEGMENTER.segment(str);
		let count = 0;
		
		for (const segment of segments) {
			count++;
		}
		
		return count;
	}
}
