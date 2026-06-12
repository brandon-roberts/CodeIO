"""
Manages gRPC channel connections to all CodeIO services.
Reads endpoint config from environment or codeio.toml.
"""
from __future__ import annotations

import os
import grpc
from dataclasses import dataclass


@dataclass
class ServiceEndpoints:
    index:    str = "localhost:50052"
    spotlight: str = "localhost:50053"
    context:  str = "localhost:50054"
    depmap:   str = "localhost:50055"
    parse:    str = "localhost:50060"
    typecheck: str = "localhost:50061"
    vm:       str = "localhost:50050"
    meta:     str = "localhost:50070"

    @classmethod
    def from_env(cls) -> "ServiceEndpoints":
        return cls(
            index     = os.getenv("CODEIO_INDEX_ADDR",     cls.index),
            spotlight = os.getenv("CODEIO_SPOTLIGHT_ADDR", cls.spotlight),
            context   = os.getenv("CODEIO_CONTEXT_ADDR",   cls.context),
            depmap    = os.getenv("CODEIO_DEPMAP_ADDR",    cls.depmap),
            parse     = os.getenv("CODEIO_PARSE_ADDR",     cls.parse),
            typecheck = os.getenv("CODEIO_TYPECHECK_ADDR", cls.typecheck),
            vm        = os.getenv("CODEIO_VM_ADDR",        cls.vm),
            meta      = os.getenv("CODEIO_META_ADDR",      cls.meta),
        )


class ChannelPool:
    """Lazy gRPC channel pool. Channels are created on first use."""

    def __init__(self, endpoints: ServiceEndpoints | None = None) -> None:
        self.ep = endpoints or ServiceEndpoints.from_env()
        self._channels: dict[str, grpc.Channel] = {}

    def channel(self, addr: str) -> grpc.Channel:
        if addr not in self._channels:
            self._channels[addr] = grpc.insecure_channel(addr)
        return self._channels[addr]

    @property
    def index(self) -> grpc.Channel:
        return self.channel(self.ep.index)

    @property
    def spotlight(self) -> grpc.Channel:
        return self.channel(self.ep.spotlight)

    @property
    def context(self) -> grpc.Channel:
        return self.channel(self.ep.context)

    @property
    def depmap(self) -> grpc.Channel:
        return self.channel(self.ep.depmap)

    @property
    def parse(self) -> grpc.Channel:
        return self.channel(self.ep.parse)

    def close(self) -> None:
        for ch in self._channels.values():
            ch.close()
        self._channels.clear()


# Module-level default pool
_default_pool: ChannelPool | None = None

def get_pool() -> ChannelPool:
    global _default_pool
    if _default_pool is None:
        _default_pool = ChannelPool()
    return _default_pool
