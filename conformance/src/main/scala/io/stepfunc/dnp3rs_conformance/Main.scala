package io.stepfunc.dnp3rs_conformance

object Main {
  def main(args: Array[String]): Unit = {
    com.automatak.dnp4s.conformance.Main.run(args, new Dnp3rsIntegrationPlugin())
  }
}
