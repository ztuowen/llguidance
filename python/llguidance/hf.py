from typing import List, Optional
from ._lib import LLTokenizer

import transformers

def from_tokenizer(
    hf_tokenizer: transformers.PreTrainedTokenizerBase,
    n_vocab: Optional[int] = None,
) -> LLTokenizer:
    if isinstance(hf_tokenizer, transformers.PreTrainedTokenizerFast):
        # this is not ideal...
        s = hf_tokenizer.backend_tokenizer.to_str()
        return LLTokenizer(s, n_vocab=n_vocab)
    else:
        raise ValueError("Only fast tokenizers are supported")
