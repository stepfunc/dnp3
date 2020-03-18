package dev.gridio.dnp3.codegen.model

trait StaticVariation extends GroupVariation {
  def staticEnumName: String
}

object StaticVariation {

  trait Binary extends StaticVariation {
    override def staticEnumName: String = "StaticBinaryVariation"
  }

  trait DoubleBinary extends StaticVariation {
    override def staticEnumName: String = "StaticDoubleBinaryVariation"
  }

  trait Analog extends StaticVariation {
    override def staticEnumName: String = "StaticAnalogVariation"
  }

  trait Counter extends StaticVariation {
    override def staticEnumName: String = "StaticCounterVariation"
  }

  trait FrozenCounter extends StaticVariation {
    override def staticEnumName: String = "StaticFrozenCounterVariation"
  }

  trait BinaryOutputStatus extends StaticVariation {
    override def staticEnumName: String = "StaticBinaryOutputStatusVariation"
  }

  trait AnalogOutputStatus extends StaticVariation {
    override def staticEnumName: String = "StaticAnalogOutputStatusVariation"
  }

  trait TimeAndInterval extends StaticVariation {
    override def staticEnumName: String = "StaticTimeAndIntervalVariation"
  }

}

