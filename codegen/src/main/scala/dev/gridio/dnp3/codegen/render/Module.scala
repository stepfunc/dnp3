package dev.gridio.dnp3.codegen.render

trait Module {
  def lines(implicit indentation: Indentation) : Iterator[String]
}
