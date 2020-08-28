package dev.gridio.dnp3.codegen.render

trait Module {
  def lines(implicit indentation: Indentation) : Iterator[String]

  def ++(other: Module) : Module = Concat(this, other)
}

final case class Concat(left: Module, right: Module) extends Module {
  def lines(implicit indentation: Indentation) : Iterator[String] = {
     left.lines ++ space ++ right.lines
  }
}
