from typing import List, Tuple, Dict, Any, Optional

import torch
import numpy as np
import pytest
import json

from llguidance.torch import (
    apply_token_bitmask_inplace,
    get_bitmask_shape,
    fill_next_token_bitmask,
    allocate_token_bitmask,
)
from llguidance import LLInterpreter, LLTokenizer

import llguidance.hf
import transformers


def _build_tokenizer():
    hf_tok = transformers.AutoTokenizer.from_pretrained(
        "unsloth/Meta-Llama-3.1-8B-Instruct"
    )
    return llguidance.hf.from_tokenizer(hf_tok)


_tokenizer: Optional[LLTokenizer] = None


def tokenizer() -> LLTokenizer:
    global _tokenizer
    if _tokenizer is None:
        _tokenizer = _build_tokenizer()
    return _tokenizer


def lark_interp(grm: str):
    gstr = json.dumps({"grammars": [{"lark_grammar": grm}]})
    interp = LLInterpreter(
        tokenizer(), gstr, enable_backtrack=False, enable_ff_tokens=False, log_level=1
    )
    interp.start_without_prompt()
    return interp


def test_grammar():
    t = tokenizer()
    mask = allocate_token_bitmask(2, t.vocab_size)
    interp = lark_interp(r"start: /[A-Z ]*/")
    fill_next_token_bitmask(interp, mask)
    allowed = []
    for idx, v in enumerate(mask[0, :].tolist()):
        for bit_idx in range(32):
            tok_idx = idx * 32 + bit_idx
            if v & (1 << bit_idx):
                if t.is_special_token(tok_idx):
                    continue
                s = t.decode_str([tok_idx])
                for c in s:
                    assert c.isupper() or c.isspace()
                allowed.append(tok_idx)
    assert len(allowed) > 100
    bt, toks = interp.commit_token(allowed[3])
    assert bt == 0
    assert toks == [allowed[3]]
    fill_next_token_bitmask(interp, mask, 1)
    assert torch.isclose(mask[1, :], mask[0, :]).all()


if __name__ == "__main__":
    test_grammar()
