#!/usr/bin/env node

import * as htmlApi from './pkg/wp_html_api_wasm.js';

const {WP_HTML_Tag_Processor} = htmlApi;

import fs from 'node:fs';
import { performance } from 'node:perf_hooks';
const html = fs.readFileSync('../../html-standard.html','utf8');

const processor = new WP_HTML_Tag_Processor(new TextEncoder().encode(html))

let c = 0;
const start = performance.now();
while ( processor.next_token() ) {
  c++;
}
const done = performance.now();
console.log(`Processed ${c} tokens in ${done-start}ms`);
