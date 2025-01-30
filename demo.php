<?php

declare(strict_types=1);

$html = <<<'HTML'
<p>Hello world!</p>
<script>
a script
</script>
<h1>You made it!</h1>
<!-- look at this comment -->
HTML;

//$html = '<p>Hello world!</p>';
$processor = new WP_HTML_Tag_Processor($html);

var_dump( $html );
while ($processor->next_token()) {
	var_dump( $processor->get_token_type() );
}
