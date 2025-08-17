# RustPOS

RustPOS is a simple Point of Sale system written in Rust. It consists of a REST API backend and a web frontend using webassembly.
This project is a functional proof of concept and not meant to be used in a production environment.
For one, the backend does not use any form of authentication yet. This project also likely does not meet the requirements of your fiscal authorities.
This is a bare-minimum implementation using modern technologies. It was born out of my frustration with free and open source POS software often being very brittle and relying on heavyweight technology stacks.

## Features

* All-Rust solution
* Leptos and Webassembly powered web UI, no javascript
* Simple REST API
* SQLite database
* Completely configurable categories and items for sale via web UI
* Supports running tabs
* Change calculation
* Sales report generation: day, month, and custom date range reports

## Feature wishlist

* POS printer support
* Kitchen printer support
* User roles: Admin, Sales, Reporting
* Inventory tracking

## Setup

### Install prerequisites
```
# Install Trunk (build tool for Rust WASM apps)
cargo install trunk

# Install WASM target
rustup target add wasm32-unknown-unknown
```


### Production Deployment

NOTE: This is a proof of concept and not ready for production use.

The build script ```build.sh``` will build frontend and backend, and copy all the neccessary files into the "rustpos" subdirectory. 
You can copy that directory anywhere you want for a more permanent installation.

```
# Build frontend and backend
./build.sh

# Run application
cd rustpos
./rustpos
```

### Run Frontend and Backend separately

To run the frontend, run ```trunk serve``` in the frontend directory. The web UI will be available at http://localhost:8080
The backend can be started via ```cargo run``` in the backend directory.

Note: The two components are preset to expect to run on the same machine. 
Theoretically you could separate the backend and frontend, and you could even expose them on a network. 
However this is not recommended unless authentication and TLS are implemented.

# RustPOS Backend

The backend uses sqlite to store the data and exposes a range of API functions for managing items, categories, and executing sales.

## API Endpoints

### Categories:

```
GET /api/categories - List all categories
POST /api/categories - Create category
GET /api/categories/:id - Get specific category
PUT /api/categories/:id - Update category
DELETE /api/categories/:id - Delete category
```

### Items:

```
GET /api/items - List all items
POST /api/items - Create item
GET /api/items/:id - Get specific item
PUT /api/items/:id - Update item
DELETE /api/items/:id - Delete item
GET /api/items/category/:category_id - Get items by category
```

### Transactions:

```
GET /api/transactions - List all transactions
POST /api/transactions - Start new transaction
GET /api/transactions/:id - Get transaction details
POST /api/transactions/:id/items - Add item to transaction
DELETE /api/transactions/:id/items/:item_id - Remove item
POST /api/transactions/:id/close - Close transaction (execute sale)
POST /api/transactions/:id/cancel - Cancel transaction
GET /api/transactions/open - Get all open transactions
```

### Sales Report Endpoint

```
POST /api/reports/sales - Generate a custom sales report for any date range
GET /api/reports/daily - Get today's sales report
GET /api/reports/monthly - Get the last 30 days sales report
```

## Example API calls:

```
# Create a category
curl -X POST http://localhost:3000/api/categories \
  -H "Content-Type: application/json" \
  -d '{"name": "Beverages", "description": "Hot and cold drinks"}'

# Check it worked
curl http://localhost:3000/api/categories

# Create an item
curl -X POST http://localhost:3000/api/items \
  -H "Content-Type: application/json" \
  -d '{"name": "Coffee", "price": 3.50, "category_id": "..."}'

# Start a transaction
curl -X POST http://localhost:3000/api/transactions \
  -H "Content-Type: application/json" \
  -d '{"customer_name": "John Doe"}'

# Add item to transaction
curl -X POST http://localhost:3000/api/transactions/{id}/items \
  -H "Content-Type: application/json" \
  -d '{"item_id": "...", "quantity": 2}'

# Close transaction
curl -X POST http://localhost:3000/api/transactions/{id}/close \
  -H "Content-Type: application/json" \
  -d '{"paid_amount": 10.00}'

# Generate a custom date range report
curl -X POST http://localhost:3000/api/reports/sales \
  -H "Content-Type: application/json" \
  -d '{
    "start_date": "2025-01-01T00:00:00Z",
    "end_date": "2025-01-31T23:59:59Z"
  }'

# Get daily report (last 24 hours)
curl http://localhost:3000/api/reports/daily

# Get monthly report (last 30 days)
curl http://localhost:3000/api/reports/monthly
```

