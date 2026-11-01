class ValidationError extends Error {
  constructor(field) {
    super("invalid " + field);
    this.name = "ValidationError";
    this.field = field;
  }
}
throw new ValidationError("email");
