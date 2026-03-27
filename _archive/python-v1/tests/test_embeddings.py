"""Test per il modulo embeddings."""

from cde.lsh import MinHashLSH


def test_lsh_insert_and_query():
    lsh = MinHashLSH(threshold=0.5)
    lsh.insert("doc1", "La temperatura a Roma è molto alta oggi")
    lsh.insert("doc2", "La temperatura a Roma è molto alta oggi con sole")
    lsh.insert("doc3", "Il gatto dorme sul divano da ore")

    results = lsh.query("La temperatura a Roma è alta oggi")
    # doc1 e doc2 dovrebbero essere candidati, doc3 no
    found_ids = [r[0] for r in results]
    assert "doc1" in found_ids or "doc2" in found_ids


def test_lsh_empty():
    lsh = MinHashLSH()
    results = lsh.query("qualcosa")
    assert len(results) == 0
