hello: int = 12
goodbye: int = 13


def hi(i: int):
    print(hello)
    hi: int = 14

    def blah():
        print(hi)

    return blah()


hi(12)
