import 'package:flutter/material.dart';

import '../controller.dart';
import '../src/rust/engine.dart';

/// The backup check: 26 multiple-choice questions answered from the paper backup (the key number,
/// then each of the 25 words). A wrong answer changes nothing; the same question stays up.
class QuizScreen extends StatefulWidget {
  const QuizScreen({super.key, required this.controller, required this.keyName});
  final SignerController controller;
  final String keyName;

  @override
  State<QuizScreen> createState() => _QuizScreenState();
}

class _QuizScreenState extends State<QuizScreen> {
  QuizQuestion? question;
  String? error;
  int? flashed;
  bool? flashedCorrect;
  bool busy = false;

  @override
  void initState() {
    super.initState();
    widget.controller.quizQuestion().then((q) {
      if (mounted) setState(() => question = q);
    }).catchError((Object e) {
      if (mounted) setState(() => error = '$e');
    });
  }

  Future<void> _answer(int choice) async {
    if (busy) return;
    setState(() {
      busy = true;
      flashed = choice;
      flashedCorrect = null;
    });
    try {
      final a = await widget.controller.quizAnswer(choice);
      if (!mounted) return;
      setState(() => flashedCorrect = a.correct);
      await Future<void>.delayed(Duration(milliseconds: a.correct ? 350 : 600));
      if (!mounted) return;
      if (a.done != null) {
        Navigator.pop(context, a.done);
        return;
      }
      setState(() {
        if (a.next != null) question = a.next;
        flashed = null;
        flashedCorrect = null;
      });
    } catch (e) {
      if (mounted) {
        setState(() {
          error = '$e';
          flashed = null;
        });
      }
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final q = question;
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(title: Text('Check backup · ${widget.keyName}')),
      body: q == null
          ? Center(child: error == null ? const CircularProgressIndicator() : Text(error!))
          : ListView(padding: const EdgeInsets.all(24), children: [
              LinearProgressIndicator(value: q.screen / q.total),
              const SizedBox(height: 8),
              Text('${q.screen + 1} of ${q.total}', textAlign: TextAlign.center, style: TextStyle(color: scheme.outline)),
              const SizedBox(height: 16),
              Text(q.title, textAlign: TextAlign.center, style: Theme.of(context).textTheme.headlineSmall),
              const SizedBox(height: 8),
              const Text('Look at your paper backup and pick what it says.', textAlign: TextAlign.center),
              const SizedBox(height: 24),
              for (var i = 0; i < q.options.length; i++)
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: SizedBox(
                    height: 64,
                    child: FilledButton.tonal(
                      style: FilledButton.styleFrom(
                        backgroundColor: flashed == i
                            ? (flashedCorrect == null
                                ? null
                                : flashedCorrect!
                                    ? Colors.green.shade600
                                    : scheme.error)
                            : null,
                        foregroundColor: flashed == i && flashedCorrect != null ? Colors.white : null,
                      ),
                      onPressed: busy ? null : () => _answer(i),
                      child: Text(q.options[i], style: const TextStyle(fontFamily: 'monospace', fontSize: 22, fontWeight: FontWeight.bold)),
                    ),
                  ),
                ),
              if (error != null) Text(error!, style: TextStyle(color: scheme.error)),
            ]),
    );
  }
}
