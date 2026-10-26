import unittest

import run


class Bench02StatisticsTest(unittest.TestCase):
    def test_percentile_uses_nearest_rank_and_p99_is_max_at_twenty(self):
        values = list(range(1, 21))
        self.assertEqual(run.percentile(values, 0.50), 10)
        self.assertEqual(run.percentile(values, 0.99), 20)

    def test_paired_bootstrap_is_deterministic_and_reports_a_ci(self):
        differences = [0.001] * 12
        first = run.bootstrap_median_ci(differences, 99)
        second = run.bootstrap_median_ci(differences, 99)
        self.assertEqual(first, second)
        self.assertEqual(first, [0.001, 0.001])

    def test_bootstrap_requires_at_least_two_pairs(self):
        self.assertIsNone(run.bootstrap_median_ci([0.1], 1))


if __name__ == "__main__":
    unittest.main()
