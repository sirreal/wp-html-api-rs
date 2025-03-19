import fs from "node:fs";
import assert from "node:assert";
import {
	WP_HTML_Processor,
	WP_HTML_Tag_Processor,
} from "../../../pkg-node/wp_html_api_wasm.js";
import { Bench } from "tinybench";
import { withCodSpeed } from "@codspeed/tinybench-plugin";
const html = fs.readFileSync(
	new URL("../../../data/html-standard.html", import.meta.url),
	"utf8",
);
const bench = withCodSpeed(
	new Bench({
		throws: true,
	}),
);

let htmlProcessor;
bench.add(
	"WP_HTML_Processor",
	() => {
		htmlProcessor = WP_HTML_Processor.create_full_parser(html);

		let c = 0;
		while (htmlProcessor.next_token()) {
			c++;
		}
		assert(c === 1_040_654);
	},
	{
		afterEach() {
			htmlProcessor.free();
		},
	},
);

let tagProcessor;
bench.add(
	"WP_HTML_Tag_Processor",
	() => {
		tagProcessor = new WP_HTML_Tag_Processor(html);

		let c = 0;
		while (tagProcessor.next_token()) {
			c++;
		}
		assert(c === 938_062);
	},
	{
		afterEach() {
			tagProcessor.free();
		},
	},
);

await bench.run();
console.table(bench.table());
