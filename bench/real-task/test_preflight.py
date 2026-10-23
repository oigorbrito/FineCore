import unittest
from unittest.mock import patch
import preflight

class PreflightTests(unittest.TestCase):
    def test_remote_rejected(self):
        for url in ("https://127.0.0.1:11434","http://localhost:11434","http://8.8.8.8:11434","http://127.0.0.1:11434/v1","http://user:pass@127.0.0.1:11434"):
            with self.subTest(url=url), self.assertRaises(ValueError):
                preflight.local_url(url)
    def test_loopback_accepted(self):
        self.assertEqual(preflight.local_url("http://127.0.0.1:11434"),"http://127.0.0.1:11434")
    def test_no_model_no_inference(self):
        with patch("preflight.request",return_value={"models":[]}) as req:
            with self.assertRaises(RuntimeError):
                preflight.preflight("http://127.0.0.1:11434","missing")
            req.assert_called_once()
    def test_plain_text_json_is_not_tool_call(self):
        tag={"models":[{"name":"sample","digest":"sha256:abcd"}]}
        plain={"message":{"content":'{"name":"write_file"}'}}
        with patch("preflight.request",side_effect=[tag,plain,plain,plain]):
            self.assertFalse(preflight.preflight("http://127.0.0.1:11434","sample")["eligible"])
    def test_three_structured_calls(self):
        tag={"models":[{"name":"sample","digest":"sha256:abcd"}]}
        good={"message":{"tool_calls":[{"function":{"name":"write_file","arguments":{"path":"hello.txt","content":"hello"}}}]}}
        with patch("preflight.request",side_effect=[tag,good,good,good]):
            self.assertTrue(preflight.preflight("http://127.0.0.1:11434","sample")["eligible"])
if __name__ == "__main__":
    unittest.main()
