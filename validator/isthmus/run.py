#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0

import os
import shutil
import subprocess
import json
import substrait_validator

def format_html_code_block(text, lang):
    text = text.replace('&', '&amp')
    text = text.replace('<', '&lt;')
    text = text.replace('>', '&gt;')
    text = text.replace('"', '&quot;')
    text = text.replace("'", '&apos;')
    text = text.replace(" ", '&nbsp;')
    text = text.replace("\t", '&nbsp;&nbsp;&nbsp;&nbsp;')
    text = text.replace("\n", '<br/>\n')
    return f'<pre><code class="language-{lang}">{text}</code></pre>'

_EXTRA_HEAD = """
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.5.0/styles/default.min.css">
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.5.0/highlight.min.js"></script>
"""

if __name__ == '__main__':

    # Figure out environment.
    script_path = os.path.dirname(os.path.realpath(__file__))
    output_path = os.path.join(script_path, 'output')
    repo_path = os.path.realpath(os.path.join(script_path, '..', '..'))
    java_path = os.path.join(repo_path, 'java')
    tpch_path = os.path.join(java_path, 'isthmus', 'src', 'test', 'resources', 'tpch')
    schema_path = os.path.join(tpch_path, 'schema.sql')
    query_path = os.path.join(tpch_path, 'queries')
    isthmus_path = os.path.join(java_path, 'isthmus', 'build', 'graal', 'isthmus')

    # Clear output directory.
    if os.path.isdir(output_path):
        shutil.rmdir(output_path)
    os.makedirs(output_path)

    # Build isthmus if it hasn't already been built.
    if not os.path.isfile(isthmus_path):
        cur = os.curdir
        os.chdir(java_path)
        try:
            subprocess.run(['./gradlew', 'nativeImage'], check=True)
        finally:
            os.chdir(cur)

    # Load TPC-H schema files.
    isthmus_args = [isthmus_path]
    with open(schema_path, 'r') as f:
        for query in filter(bool, map(str.strip, f.read().split(';'))):
            isthmus_args.append('-c')
            isthmus_args.append(query)

    # Run isthmus and the validator for all queries.
    for query_fname in sorted(os.listdir(query_path)):
        name = query_fname.split('.')[0]
        try:

            # Read query.
            with open(os.path.join(query_path, query_fname), 'r') as f:
                query = f.read()

            # Convert to Substrait plan with Isthmus.
            plan = subprocess.run(isthmus_args + [query], check=True, capture_output=True).stdout.decode('utf-8')

            # Convert to HTML validation result with validator.
            html = substrait_validator.plan_to_html(plan)

            # Unpack the HTML a bit so we can add stuff to it.
            html_gen_a, remain = html.split('</head>', maxsplit=1)
            html_gen_b, remain = remain.split('<body>', maxsplit=1)
            html_gen_b = f'</head>{html_gen_b}<body>'
            html_gen_c, html_gen_d = remain.split('</body>', maxsplit=1)
            html_gen_d = f'</body>{html_gen_d}'

            # Add our stuff to it.
            html = []
            html.append(html_gen_a)
            html.append(f'<title>{name}</title>')
            html.append(_EXTRA_HEAD)
            html.append(html_gen_b)
            html.append(f'<h1>{name} with Isthmus</h1>')
            html.append(f'<h2>SQL query</h2>')
            html.append(format_html_code_block(query, 'sql'))
            html.append(f'<h2>Plan JSON</h2>')
            html.append(format_html_code_block(plan, 'json'))
            html.append(f'<h2>Validation result</h2>')
            html.append(html_gen_c)
            html.append('<script>hljs.highlightAll();</script>')
            html.append(html_gen_d)
            html = '\n'.join(html)

            # Write file.
            html_fname = os.path.join(output_path, f'{name}.html')
            with open(html_fname, 'w') as f:
                f.write(html)

        except Exception as e:
            print(f'{type(e).__name__} for {name}: {e}')
