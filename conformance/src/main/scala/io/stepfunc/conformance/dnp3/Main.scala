package io.stepfunc.conformance.dnp3

object Main {
  def main(args: Array[String]): Unit = {
    io.stepfunc.dnp3c.conformance.Main.run(args, new Dnp3IntegrationPlugin())
  }
}
