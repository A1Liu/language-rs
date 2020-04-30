# Ideas to Include in a new Programming Language
Focus is on programmer ergonomics. Should be as easy to use as python and as fast
as Java.

1. Heavy type inference; type inference of variables, and also of function parameters
   and return type; interfaces are made for every individual method, and also for
   every method combination that is inferred. Classes don't inherit from other classes,
   instead they inherit from interfaces, and the implementation is copied over.
2. Python syntax
3. Compile-time errors are reported but only stop execution if they're on a potential
   runtime path.
4. Error-reporting can be turned on and off again at will
5. Casts are inserted whenever necessary, but emit a warning.
0. Class meta-types
1. Match statements; supports match on class of object
2. Generators, coroutines, asyncio, all supported
3. implicit return of None
4. Shadowing is not allowed, except accross function boundaries
5. Variables are implicitly initialized to `None`
6. If, Else, etc. do not produce a new scope.
7. No recursive imports; statically checked.
8. Types cannot be reassigned.

## Examples

```python
def function():
  pass

print("Hello, world!")

class A()<In, Out>:
  pass
```

Should translate to the following java code:

```java
public class Script {
  public class Function0 {
    public Object call() {
      return null;
    }
  }

  public class A<In, Out> {}

  public static Function0 function;
  public static Class<? extends Object> classA;

  public static void main() {
    function = new Function0();

    System.out.println("Hello, world!");

    classA = A.class;
  }
}
```

