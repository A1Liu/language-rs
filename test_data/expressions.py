asdf: float = 12.0
hello: float = 12.0
meep: float = asdf + hello


def add(a1: float, a2: float, a3: float) -> float:
    print(a2 + a2)

    def add2(a1: float) -> float:
        return a1

    return add2(a1) + a2 + a3


def hi(asdf: float) -> float:
    print(asdf)
    print(asdf)


hi(12.0)

print(add(12.0, 12.1, 12.2))
