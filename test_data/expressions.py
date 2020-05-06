asdf: float = 12.0
hello: float = 12.0
meep: float = asdf + hello


def add(a1: float, a2: float, a3: float) -> float:
    print(a2 + a2)


def hi(asdf: float) -> float:
    print(asdf)
    print(asdf)


hi(12.0)

add(12.0, 12.1, 12.2)
