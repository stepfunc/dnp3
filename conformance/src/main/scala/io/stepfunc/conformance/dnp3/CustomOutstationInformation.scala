package io.stepfunc.conformance.dnp3

import io.stepfunc.dnp3.{BroadcastAction, FunctionCode, OutstationInformation, RequestHeader}
import org.joou.UByte

class CustomOutstationInformation extends OutstationInformation {
  override def processRequestFromIdle(requestHeader: RequestHeader): Unit = {}

  override def broadcastReceived(functionCode: FunctionCode, broadcastAction: BroadcastAction): Unit = {}

  override def enterSolicitedConfirmWait(uByte: UByte): Unit = {}

  override def solicitedConfirmTimeout(uByte: UByte): Unit = {}

  override def solicitedConfirmReceived(uByte: UByte): Unit = {}

  override def solicitedConfirmWaitNewRequest(): Unit = {}

  override def wrongSolicitedConfirmSeq(uByte: UByte, uByte1: UByte): Unit = {}

  override def unexpectedConfirm(b: Boolean, uByte: UByte): Unit = {}

  override def enterUnsolicitedConfirmWait(uByte: UByte): Unit = {}

  override def unsolicitedConfirmTimeout(uByte: UByte, b: Boolean): Unit = {}

  override def unsolicitedConfirmed(uByte: UByte): Unit = {}

  override def clearRestartIin(): Unit = {}
}
