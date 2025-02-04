#!/usr/bin/env python3

# For standalone use, please use this:
# https://github.com/guidance-ai/llguidance/blob/main/python/llguidance/gbnf_to_lark.py

import os
import sys

sys.path.insert(
    0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../python"))
)

from llguidance.gbnf_to_lark import main

main()
