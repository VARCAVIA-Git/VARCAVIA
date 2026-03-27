"""
ONNX Runtime Wrapper — Configurazione ottimizzata per CPU.
"""

import os
from typing import Optional


def get_session_options():
    """Restituisce opzioni di sessione ONNX ottimizzate per CPU."""
    import onnxruntime as ort

    opts = ort.SessionOptions()
    opts.intra_op_num_threads = min(os.cpu_count() or 4, 4)  # Max 4 threads per laptop
    opts.inter_op_num_threads = 1
    opts.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
    opts.enable_mem_pattern = True
    opts.enable_cpu_mem_arena = True
    return opts
