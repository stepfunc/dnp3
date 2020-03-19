package dev.gridio.dnp3.codegen.render

object SpacedIndent extends Indentation {
  override def apply(f: => Iterator[String]): Iterator[String] = f.map(s => s"    $s")
}
