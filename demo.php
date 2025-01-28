<?php

declare(strict_types=1);


$processor = \WP_HTML_Processor_RS::create_fragment('<p>Hello world!</p>');

var_dump($processor);
while ($processor->next_token()) {
	var_dump( $processor->get_token_type() );
}
