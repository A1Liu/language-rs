def add1(a: int) -> int:
    if a:
        return add1(a - 1) + 1
    else:
        return 1


print(add1(12))
