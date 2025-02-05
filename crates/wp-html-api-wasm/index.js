#!/usr/bin/env node

import * as htmlApi from './pkg/wp_html_api_wasm.js';

const {WP_HTML_Tag_Processor} = htmlApi;

const processor = new WP_HTML_Tag_Processor(new TextEncoder().encode(`<div></div><style>
body>*::before {
content: attr(fallback);
}
</style>
`))

processor.print_bytes();

while ( processor.next_token() ) {
  console.log( processor.get_token_type() );
  console.log( new TextDecoder().decode(processor.token()) );
  console.log( "%o", new TextDecoder().decode(processor.get_tag()) );
}
