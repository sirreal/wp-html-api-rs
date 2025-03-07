#!/usr/bin/env php -d extension=target/release/libwp_html_api_php_ext.dylib
<?php

declare(strict_types=1);

// Buffer output to reduce impact of printing on timings.
ob_start();

$opts = getopt('i:v', ['color::']);
$use_color  = match ( $opts['color'] ?? null ) {
    'never'         => false,
    'always', false => true,
    'auto', null    => posix_isatty(STDOUT),
};
$input_file = $opts['i'] ?? null;
$is_verbose = isset( $opts['v'] );

if ( null === $input_file ) {
    $html = <<<'HTML'
        <title lang=en-US> <titl> foo bar & great <titl></tItLe>
        <p class='separator introduction' =5>Hello world!</p>
        <script>
        a script
        </script>
        <h1 name="title">You made it!</h1>
        <!-- look at this comment -->
        HTML;
} else {
    $html = file_get_contents( $input_file );
}

$make_html_processor = function() use (&$html) {
	return WP_HTML_Processor::create_full_parser($html);
};
$make_tag_processor = function() use (&$html) {
	return new WP_HTML_Tag_Processor($html);
};

foreach ([
	[ WP_HTML_Processor::class, $make_html_processor ],
	[ WP_HTML_Tag_Processor::class, $make_tag_processor ],
] as [ $class, $make_processor ]) {
	ob_flush();
	$processor = $make_processor();
	$c = 0;
	$ns = -hrtime( true );
	if ( $is_verbose ) {
		while ($processor->next_token()) {
			$c++;

			switch ( $processor->get_token_type() ) {
				case '#tag':
					echo $use_color
						? "\e[2;35m<\e[0;34m{$processor->get_token_name()}\e[2;35m>\e[m\n"
						: "<{$processor->get_token_name()}>\n";
					break;

				case '#text':
					echo $use_color ? "\e[2;90m#text\e[m\n" : "#text\n";
					break;
			}
		}
	} else {
		while ($processor->next_token()) {
			$c++;
		}
	}
	$ns  += hrtime( true );
	$n    = new NumberFormatter( 'en-US', NumberFormatter::DEFAULT_STYLE );
	$ms   = $n->format( $ns / 1e6 );
	$mbps = $n->format( strlen( $html ) / 1e6 / ( $ns / 1e9 ) );
	$c    = $n->format( $c );

	echo "With {$class}\n";
	if ( $use_color ) {
		echo "\t\e[90mTook \e[33m{$ms}\e[2mms\e[0;90m (\e[34m{$mbps}\e[2mMB/s\e[0;90m)\e[m\n";
		echo "\t\e[90mFound \e[36m{$c}\e[90m tokens.\e[m\n";
	} else {
		echo "\tTook {$ms}ms ({$mbps}MB/s)\n";
		echo "\tFound {$c} tokens.\n";
	}
}
