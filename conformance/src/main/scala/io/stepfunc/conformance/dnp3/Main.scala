package io.stepfunc.conformance.dnp3

object Main {
  def main(args: Array[String]): Unit = {
    io.stepfunc.dnp4s.conformance.Main.run(args, new Dnp3IntegrationPlugin())
  }
}
