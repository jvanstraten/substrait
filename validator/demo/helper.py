# SPDX-License-Identifier: Apache-2.0

import os
import subprocess

# Figure out environment.
script_path = os.path.dirname(os.path.realpath(__file__))
repo_path = os.path.realpath(os.path.join(script_path, '..', '..'))
java_path = os.path.join(repo_path, 'java')
tpch_path = os.path.join(java_path, 'isthmus', 'src', 'test', 'resources', 'tpch')
schema_path = os.path.join(tpch_path, 'schema.sql')
query_path = os.path.join(tpch_path, 'queries')
isthmus_path = os.path.join(java_path, 'isthmus', 'build', 'graal', 'isthmus')

# Load TPC-H schema files.
isthmus_args = [isthmus_path]
with open(schema_path, 'r') as f:
    for query in filter(bool, map(str.strip, f.read().split(';'))):
        isthmus_args.append('-c')
        isthmus_args.append(query)

def run_isthmus_with_tpch(query):
    return subprocess.run(isthmus_args + [query], check=True, capture_output=True).stdout.decode('utf-8')

def get_tpch(i):
    with open(os.path.join(query_path, f'{i:02d}.sql'), 'r') as f:
        return f.read()
