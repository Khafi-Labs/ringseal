import 'dart:async';
import 'package:flutter/material.dart';
import '../client.dart';
import '../models.dart';
import 'ringseal_theme.dart';

/// A pre-built verification screen widget that banks can customize and embed.
class RingsealVerificationScreen extends StatefulWidget {
  final RingsealClient client;
  final RingsealTheme? theme;
  final VoidCallback? onSessionVerified;
  final VoidCallback? onNoActiveSession;
  final Widget? loadingWidget;
  final Widget? noSessionWidget;

  const RingsealVerificationScreen({
    Key? key,
    required this.client,
    this.theme,
    this.onSessionVerified,
    this.onNoActiveSession,
    this.loadingWidget,
    this.noSessionWidget,
  }) : super(key: key);

  @override
  State<RingsealVerificationScreen> createState() => _RingsealVerificationScreenState();
}

class _RingsealVerificationScreenState extends State<RingsealVerificationScreen> {
  Timer? _pollingTimer;
  Timer? _countdownTimer;
  
  VerificationSession? _session;
  bool _isLoading = true;
  String? _error;
  Duration _timeLeft = Duration.zero;

  @override
  void initState() {
    super.initState();
    _startPolling();
    // TODO: Call platform-specific capture detection via platform channel (D-01)
  }

  @override
  void dispose() {
    _pollingTimer?.cancel();
    _countdownTimer?.cancel();
    super.dispose();
  }

  void _startPolling() {
    _fetchSession();
    _pollingTimer = Timer.periodic(widget.client.config.pollingInterval, (_) {
      _fetchSession();
    });
  }

  Future<void> _fetchSession() async {
    try {
      final session = await widget.client.getActiveSession();
      if (!mounted) return;
      
      setState(() {
        _isLoading = false;
        _error = null;
        
        if (session == null) {
          _session = null;
          widget.onNoActiveSession?.call();
        } else {
          final previousStatus = _session?.status;
          _session = session;
          
          if (session.status == SessionStatus.verified && previousStatus != SessionStatus.verified) {
            widget.onSessionVerified?.call();
          }
          
          _updateCountdown();
        }
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _isLoading = false;
        _error = e.toString();
      });
    }
  }

  void _updateCountdown() {
    if (_session == null) {
      _countdownTimer?.cancel();
      return;
    }
    
    final now = DateTime.now();
    final diff = _session!.expiresAt.difference(now);
    
    if (diff.isNegative) {
      _timeLeft = Duration.zero;
      _countdownTimer?.cancel();
    } else {
      _timeLeft = diff;
      if (_countdownTimer == null || !_countdownTimer!.isActive) {
        _countdownTimer = Timer.periodic(const Duration(seconds: 1), (_) {
          if (!mounted) return;
          setState(() {
            final now = DateTime.now();
            final diff = _session!.expiresAt.difference(now);
            if (diff.isNegative) {
              _timeLeft = Duration.zero;
              _countdownTimer?.cancel();
            } else {
              _timeLeft = diff;
            }
          });
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = widget.theme ?? RingsealTheme.defaultTheme;

    if (_isLoading) {
      return Center(
        child: widget.loadingWidget ?? const CircularProgressIndicator(),
      );
    }

    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline, color: theme.warningColor, size: 48),
            const SizedBox(height: 16),
            Text(
              'Error checking status.',
              style: theme.subtitleStyle,
            ),
            Text(_error!),
          ],
        ),
      );
    }

    if (_session == null) {
      return widget.noSessionWidget ?? Center(
        child: Text('No active call verification session.', style: theme.subtitleStyle),
      );
    }

    return Container(
      color: theme.backgroundColor,
      padding: const EdgeInsets.all(24.0),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Text(
            'Call Verification',
            style: theme.titleStyle ?? TextStyle(
              fontSize: 24,
              fontWeight: FontWeight.bold,
              color: theme.primaryColor,
            ),
          ),
          const SizedBox(height: 24),
          Text(
            'Never read this code aloud. Your bank will never ask you to share your screen.',
            style: TextStyle(
              color: theme.warningColor,
              fontWeight: FontWeight.bold,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 32),
          ExcludeSemantics(
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: _session!.code.split('').map((char) => _buildCodeBox(char, theme)).toList(),
            ),
          ),
          const SizedBox(height: 48),
          Text(
            'Code expires in ${_timeLeft.inMinutes}:${(_timeLeft.inSeconds % 60).toString().padLeft(2, '0')}',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              fontSize: 16,
              color: theme.primaryColor,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildCodeBox(String char, RingsealTheme theme) {
    return Container(
      width: 48,
      height: 64,
      decoration: BoxDecoration(
        color: theme.codeBoxColor,
        borderRadius: theme.codeBoxRadius ?? BorderRadius.circular(8),
        border: Border.all(color: theme.primaryColor.withOpacity(0.3)),
      ),
      alignment: Alignment.center,
      child: Text(
        char,
        style: theme.codeStyle ?? TextStyle(
          fontSize: 32,
          fontWeight: FontWeight.bold,
          color: theme.codeTextColor,
        ),
      ),
    );
  }
}
