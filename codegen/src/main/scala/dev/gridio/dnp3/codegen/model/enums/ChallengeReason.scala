package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, Hex}

object ChallengeReason {

  private val comments = List(
    "Enumerates reasons for a sec-auth challenge"
  )

  def apply(): EnumModel = EnumModel("ChallengeReason", comments, EnumModel.UInt8, codes, Some(defaultValue), Hex)

  private val defaultValue = EnumValue("UNKNOWN", 255, "Unknown reason")

  private val codes = List(
    EnumValue("CRITICAL", 1, "Challenging a critical function")
  )

}



