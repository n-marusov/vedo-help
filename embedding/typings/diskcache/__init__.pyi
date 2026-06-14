from typing import Any

class Cache:
    def __init__(
        self,
        directory: str | None = None,
        timeout: int = 60,
        disk: type = ...,  # type: ignore[valid-type]
        **settings: Any,
    ) -> None: ...
    def get(
        self,
        key: str,
        default: object = None,
        read: bool = False,
        expire_time: bool = False,
        tag: bool = False,
        retry: bool = False,
    ) -> Any: ...
    def set(
        self,
        key: str,
        value: Any,
        expire: float | None = None,
        read: bool = False,
        tag: str | None = None,
        retry: bool = False,
    ) -> None: ...
