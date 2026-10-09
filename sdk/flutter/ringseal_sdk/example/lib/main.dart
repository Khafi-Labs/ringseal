import 'package:flutter/material.dart';
import 'package:ringseal_sdk/ringseal_sdk.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Ringseal Example',
      theme: ThemeData(
        primarySwatch: Colors.blue,
      ),
      home: const BankHomeScreen(),
    );
  }
}

class BankHomeScreen extends StatefulWidget {
  const BankHomeScreen({Key? key}) : super(key: key);

  @override
  State<BankHomeScreen> createState() => _BankHomeScreenState();
}

class _BankHomeScreenState extends State<BankHomeScreen> {
  late RingsealClient _client;
  bool _isLoggedIn = false;

  @override
  void initState() {
    super.initState();
    // Initialize the SDK client with the backend API URL
    _client = RingsealClient(
      config: const RingsealConfig(
        baseUrl: 'https://api.example-bank.com/ringseal',
      ),
    );
  }

  @override
  void dispose() {
    _client.dispose();
    super.dispose();
  }

  void _login() {
    // Simulate user login
    setState(() {
      _isLoggedIn = true;
    });
    // The bank's backend issues a JWT for the customer session
    _client.setAuthToken('eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.dummy_token');
  }

  void _navigateToVerification() {
    Navigator.of(context).push(MaterialPageRoute(builder: (context) {
      return Scaffold(
        appBar: AppBar(title: const Text('Call Verification')),
        body: RingsealVerificationScreen(
          client: _client,
          // Custom theme to match the bank's styling
          theme: RingsealTheme(
            primaryColor: Colors.teal,
            codeBoxColor: Colors.teal.shade50,
            codeTextColor: Colors.teal.shade900,
            warningColor: Colors.deepOrange,
            codeBoxRadius: BorderRadius.circular(12),
          ),
          onSessionVerified: () {
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(content: Text('Agent verified successfully!')),
            );
          },
          onNoActiveSession: () {
            // Callback executed when no session is active.
          },
        ),
      );
    }));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('SecureBank App')),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            if (!_isLoggedIn)
              ElevatedButton(
                onPressed: _login,
                child: const Text('Login'),
              )
            else ...[
              const Text('Logged In'),
              const SizedBox(height: 20),
              ElevatedButton(
                onPressed: _navigateToVerification,
                child: const Text('Verify Caller Agent'),
              ),
            ]
          ],
        ),
      ),
    );
  }
}
