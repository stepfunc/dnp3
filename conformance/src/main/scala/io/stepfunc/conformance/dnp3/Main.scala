package io.stepfunc.conformance.dnp3

object Main {
  def main(args: Array[String]): Unit = {
    com.automatak.dnp4s.conformance.Main.run(args, new Dnp3IntegrationPlugin())
  }
}
