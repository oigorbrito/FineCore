import unittest
from select_swebench import select_rows, REQUIRED

def instance(i):
    return dict(zip(REQUIRED,[i,"org/repo","deadbeef","fix issue","patch",["t1"],["t2"]]))
class SelectionTests(unittest.TestCase):
    def test_order_independent_of_input(self):
        result=select_rows([instance("c"),instance("a"),instance("b")],2)
        self.assertEqual([r["instance_id"] for r in result["proposed_first"]],["a","b"])
        self.assertEqual(result["qualification"],"NOT_EXECUTED")
        self.assertTrue(all(r["qualification"]=="PENDING_OFFICIAL_EVALUATOR" for r in result["candidate_order"]))
    def test_reject_missing_and_duplicates(self):
        rows=[instance("a"),instance("a"),{**instance("b"),"test_patch":""}]
        result=select_rows(rows)
        self.assertEqual(result["candidate_count"],1)
        self.assertEqual(len(result["rejected"]),2)
if __name__=="__main__": unittest.main()
