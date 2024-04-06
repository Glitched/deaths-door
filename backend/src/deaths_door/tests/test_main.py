from deaths_door import main


def test_hello_world_succeeds():
    """Asserts that we return a fun message."""
    assert main.hello_world() == "Hello, World!"
