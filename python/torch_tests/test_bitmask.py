import torch
import numpy as np
import pytest

from llguidance.torch import apply_token_bitmask_inplace, get_bitmask_shape

has_cuda = torch.cuda.is_available()
dev = "cuda" if has_cuda else "cpu"


def u32_to_bool(arr: np.ndarray) -> np.ndarray:
    arr = arr.astype(np.uint32)
    bool_arr = np.unpackbits(arr.view(np.uint8), bitorder="little").reshape(
        arr.shape[0], arr.shape[1] * 32
    )
    return bool_arr


def measure_gpu_time(func, *args, **kwargs):
    start = torch.cuda.Event(enable_timing=True)
    end = torch.cuda.Event(enable_timing=True)

    torch.cuda.synchronize()  # Ensure previous ops are finished
    start.record()

    func(*args, **kwargs)  # Run the function

    end.record()
    torch.cuda.synchronize()  # Wait for all ops to finish

    return start.elapsed_time(end)  # Returns time in milliseconds


def gen_mask_test_data(batch: int, vocab: int) -> tuple[torch.Tensor, torch.Tensor]:
    data = torch.rand(batch, vocab, dtype=torch.float32, device=dev)
    mask = torch.randint(
        0, 2**32, get_bitmask_shape(batch, vocab), dtype=torch.uint32, device=dev
    )
    mask = mask.view(dtype=torch.int32)
    return data, mask


@pytest.mark.parametrize("batch", [1, 3, 10])
@pytest.mark.parametrize("vocab", list(range(32000, 32033)) + [128000])
def test_mask_data(batch: int, vocab: int):
    data, mask = gen_mask_test_data(batch, vocab)

    masked_data = data.clone()
    apply_token_bitmask_inplace(masked_data, mask.to(dtype=torch.int32))

    data_np = data.cpu().numpy()
    mask_np = mask.cpu().numpy()
    md_np = masked_data.cpu().numpy()

    bool_mask = u32_to_bool(mask_np)[:, :vocab]
    assert np.all(md_np[bool_mask == 0] == -np.inf)
    assert np.all(md_np[bool_mask == 1] == data_np[bool_mask == 1])


def test_mask_data_perf():
    if not has_cuda:
        pytest.skip("CUDA is not available")
    times = []
    for batch in [100, 105, 95, 107, 108, 99]:
        data, mask = gen_mask_test_data(batch, 128000)
        for _ in range(10):
            masked_data = data.clone()
            t = measure_gpu_time(apply_token_bitmask_inplace, masked_data, mask)
            times.append(round(t * 1000))
    print(times)


if __name__ == "__main__":
    test_mask_data_perf()
