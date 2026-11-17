#!/usr/bin/env dart

import 'dart:io';

void main(List<String> args) async {
  final isWindows = Platform.isWindows;
  final binNames = isWindows
      ? ['asep.exe', 'spec-engine.exe']
      : ['asep', 'spec-engine'];

  final scriptFile = File(Platform.script.toFilePath());
  final scriptDir = scriptFile.parent;

  String? findBinary() {
    for (final binName in binNames) {
      final candidatePaths = [
        Directory(scriptDir.path).parent.uri.resolve('bin/$binName').toFilePath(),
        Directory(scriptDir.path).parent.parent.uri.resolve('target/release/$binName').toFilePath(),
        Directory(scriptDir.path).parent.parent.uri.resolve('target/debug/$binName').toFilePath(),
      ];

      for (final p in candidatePaths) {
        if (File(p).existsSync()) {
          return p;
        }
      }

      final pathEnv = Platform.environment['PATH'] ?? '';
      final separator = isWindows ? ';' : ':';
      for (final dir in pathEnv.split(separator)) {
        if (dir.isEmpty) continue;
        final candidate = '$dir${Platform.pathSeparator}$binName';
        if (File(candidate).existsSync()) {
          return candidate;
        }
      }
    }
    return null;
  }

  String? binaryPath = findBinary();

  if (binaryPath == null) {
    try {
      final cargoCheck = Process.runSync('cargo', ['--version']);
      if (cargoCheck.exitCode == 0) {
        stderr.writeln('[asep] Native binary not found. Compiling via cargo...');
        final projectRoot = Directory(scriptDir.path).parent.parent.path;
        final build = Process.runSync(
          'cargo',
          ['build', '--release', '--bin', 'asep'],
          workingDirectory: projectRoot,
        );
        if (build.exitCode == 0) {
          binaryPath = findBinary();
        } else {
          stderr.write(build.stderr);
        }
      }
    } catch (_) {}
  }

  if (binaryPath == null) {
    stderr.writeln(
      "[asep] Error: Could not locate native executable 'asep' or 'spec-engine'.\n"
      "Please ensure Rust is installed and run 'cargo build --release' or download the precompiled binary.",
    );
    exit(1);
  }

  final process = await Process.start(
    binaryPath,
    args,
    mode: ProcessStartMode.inheritStdio,
  );

  final exitCode = await process.exitCode;
  exit(exitCode);
}

