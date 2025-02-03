from typing import Tuple
import torch
from ._lib import LLInterpreter


def get_bitmask_shape(batch_size: int, vocab_size: int) -> Tuple[int, int]:
    return (batch_size, (vocab_size + 31) // 32)


def allocate_token_bitmask(batch_size: int, vocab_size: int) -> torch.Tensor:
    return torch.full(
        get_bitmask_shape(batch_size, vocab_size),
        -1,
        dtype=torch.int32,
        pin_memory=torch.cuda.is_available(),
    )


@torch.compile(dynamic=True)  # faster than dynamic=False and jit.script
def apply_token_bitmask_inplace_kernel(logits: torch.Tensor, mask: torch.Tensor):
    mask_expanded = torch.repeat_interleave(mask, 32, dim=1)
    bit_indices = torch.arange(32, device=logits.device, dtype=torch.int32).repeat(
        mask.shape[1]
    )
    bit_masks = (mask_expanded >> bit_indices) & 1  # Extract each bit
    bit_masks = bit_masks[:, : logits.shape[1]]  # Trim to match vocab size
    logits.masked_fill_(bit_masks == 0, float("-inf"))  # Apply mask


def apply_token_bitmask_inplace(logits: torch.Tensor, mask: torch.Tensor) -> None:
    if logits.dim() == 1:
        logits = logits.unsqueeze(0)
    if mask.dim() == 1:
        mask = mask.unsqueeze(0)
    assert mask.dtype == torch.int32, "Mask must be int32"
    assert logits.dim() == 2, "Logits must be 2D"
    batch, vocab = logits.shape
    assert mask.shape == get_bitmask_shape(batch, vocab), "Mask shape mismatch"
    apply_token_bitmask_inplace_kernel(logits, mask)


def fill_next_token_bitmask(
    interp: LLInterpreter, bitmask: torch.Tensor, index: int = 0
) -> str:
    assert bitmask.dtype == torch.int32, "Mask must be int32"
    assert bitmask.is_cpu, "Mask must be on CPU"
    assert bitmask.dim() == 2, "Mask must be 2D"
    v = bitmask[index, :]
    assert bitmask.is_contiguous(), "Mask must be contiguous"
    return interp.unsafe_compute_mask_ptr(v.data_ptr(), v.numel() * v.element_size())
