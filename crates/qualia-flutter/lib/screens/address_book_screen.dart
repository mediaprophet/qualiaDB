import 'package:flutter/material.dart';

class AddressBookScreen extends StatelessWidget {
  const AddressBookScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Agreements & Consent',
            style: Theme.of(context).textTheme.headlineMedium?.copyWith(
              color: Theme.of(context).colorScheme.primary,
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 24),
          Card(
            color: Theme.of(context).colorScheme.surface,
            child: ListTile(
              leading: const Icon(Icons.handshake),
              title: const Text('Pending Ratification'),
              subtitle: const Text('2 guardians required'),
              trailing: ElevatedButton(
                onPressed: () {},
                child: const Text('Sign'),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
