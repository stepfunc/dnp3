// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
