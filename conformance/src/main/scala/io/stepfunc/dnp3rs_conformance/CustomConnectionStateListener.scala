package io.stepfunc.dnp3rs_conformance

import io.stepfunc.dnp3rs.{ConnectionState, ConnectionStateListener}

class CustomConnectionStateListener extends ConnectionStateListener {
  override def onChange(connectionState: ConnectionState): Unit = {

  }
}
