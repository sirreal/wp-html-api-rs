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

bench.add("WP_HTML_Processor", () => {
	const processor = WP_HTML_Processor.create_full_parser(html);

	let c = 0;
	while (processor.next_token()) {
		c++;
	}
	assert(c === 1_040_654);
});

bench.add("WP_HTML_Tag_Processor", () => {
	const processor = new WP_HTML_Tag_Processor(html);

	let c = 0;
	while (processor.next_token()) {
		c++;
	}
	assert(c === 938_062);
});

await bench.run();
console.table(bench.table());
