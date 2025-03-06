#!/usr/bin/env node

import * as htmlApi from "./crates/wp-html-api-wasm/pkg/wp_html_api_wasm.js";

const { WP_HTML_Tag_Processor, WP_HTML_Processor } = htmlApi;

import fs from "node:fs";
import { performance } from "node:perf_hooks";
const html = fs.readFileSync(
	new URL("./data/html-standard.html", import.meta.url),
	"utf8",
);

const $use_color = true;

function make_html_processor() {
	return WP_HTML_Processor.create_full_parser(new TextEncoder().encode(html));
}

function make_tag_processor() {
	return new WP_HTML_Tag_Processor(new TextEncoder().encode(html));
}

const fmt = new Intl.NumberFormat("en-US");
const byteLength = new TextEncoder().encode(html).byteLength;

[
	[WP_HTML_Processor.name, make_html_processor],
	[WP_HTML_Tag_Processor.name, make_tag_processor],
].forEach(([className, makeProcessor]) => {
	const processor = makeProcessor();

	let c = 0;
	const start = performance.now();
	while (processor.next_token()) {
		c++;
	}
	const done = performance.now();

	const ms = fmt.format(done - start);
	const mbps = fmt.format(byteLength / 1e6 / ((done - start) / 1e3));

	console.log(`With ${className}`);
	if ($use_color) {
		console.log(
			`\t\x1b[90mTook \x1b[33m${ms}\x1b[2mms\x1b[0;90m (\x1b[34m${mbps}\x1b[2mMB/s\x1b[0;90m)\x1b[m`,
		);
		console.log(
			`\t\x1b[90mFound \x1b[36m${fmt.format(c)}\x1b[90m tokens.\x1b[m`,
		);
	} else {
		console.log(`\tTook ${ms}ms (${mbps}MB/s)`);
		console.log(`\tFound ${fmt.format(c)} tokens.`);
	}
});
