// super.this and super.base while a super member's key is awaited, in a method.
class Base { m(x) { return "base " + x; } }
class Child extends Base {
  async m(x) { return super[await later("m")](x); }
}
new Child().m(7).then(print);
