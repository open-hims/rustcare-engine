# Dynamic Form Builder Guide

## Overview

The Dynamic Form Builder allows hospital staff (doctors, nurses, admins, management) to create custom forms for their daily workflows without writing code. Forms can be created visually through the UI and used across any module.

## Features

- **Visual Form Builder**: Drag-and-drop interface to create forms
- **14+ Field Types**: Text, email, number, phone, date, time, datetime, textarea, select, radio, checkbox, file, signature, section headers
- **Module Support**: Forms can be associated with any module (healthcare, pharmacy, billing, etc.)
- **Entity Linking**: Forms can be linked to specific entities (patient, appointment, etc.)
- **Versioning**: Forms are versioned when schema changes
- **Approval Workflows**: Forms can require approval before submission
- **Access Control**: Role and permission-based access control
- **Form Submissions**: Track and view all form submissions

## User Roles & Use Cases

### Doctors
- **Patient Intake Forms**: Collect patient information during registration
- **Medical History Forms**: Document patient medical history
- **Examination Forms**: Record examination findings
- **Prescription Forms**: Create prescription templates
- **Discharge Summaries**: Standardize discharge documentation

### Nurses
- **Vital Signs Forms**: Record temperature, blood pressure, pulse, etc.
- **Medication Administration Records**: Track medication given to patients
- **Patient Care Notes**: Document patient care activities
- **Shift Handover Forms**: Transfer patient information between shifts
- **Incident Reports**: Document incidents and near-misses

### Administrative Staff
- **Patient Registration Forms**: Collect patient demographics
- **Insurance Verification Forms**: Verify insurance coverage
- **Appointment Scheduling Forms**: Schedule and manage appointments
- **Billing Forms**: Process billing information
- **Consent Forms**: Document patient consent

### Management/CEO
- **Staff Evaluation Forms**: Evaluate staff performance
- **Compliance Checklists**: Ensure regulatory compliance
- **Quality Assurance Forms**: Track quality metrics
- **Budget Forms**: Manage department budgets
- **Audit Forms**: Conduct internal audits

## Creating a Form

1. Navigate to `/admin/forms`
2. Click "Create New Form"
3. Fill in form metadata:
   - **Form Name**: Internal name (e.g., `patient-intake-form`)
   - **Form Slug**: URL-friendly identifier (e.g., `patient-intake`)
   - **Display Name**: User-friendly name (e.g., "Patient Intake Form")
   - **Description**: What the form is used for
   - **Module**: Which module this form belongs to
   - **Category**: Optional category for organization
4. Add fields from the field palette
5. Configure each field:
   - Label, name, placeholder
   - Required/optional
   - Validation rules
   - Options (for select/radio)
   - Help text
6. Preview the form
7. Save the form

## Field Types

### Text Input
- **Use Case**: Names, addresses, general text
- **Validation**: Min/max length, pattern matching

### Email
- **Use Case**: Email addresses
- **Validation**: Email format validation

### Number
- **Use Case**: Age, weight, counts
- **Validation**: Min/max values

### Phone
- **Use Case**: Phone numbers
- **Validation**: Phone format validation

### Date
- **Use Case**: Birth dates, appointment dates
- **Format**: YYYY-MM-DD

### Time
- **Use Case**: Appointment times, medication times
- **Format**: HH:MM

### DateTime
- **Use Case**: Timestamps, scheduled events
- **Format**: YYYY-MM-DDTHH:MM

### Textarea
- **Use Case**: Notes, descriptions, long text
- **Validation**: Min/max length

### Select (Dropdown)
- **Use Case**: Single choice from options
- **Options**: Custom options list

### Radio Buttons
- **Use Case**: Single choice from visible options
- **Options**: Custom options list

### Checkbox
- **Use Case**: Yes/no, true/false, multiple selections

### File Upload
- **Use Case**: Documents, images, attachments
- **Note**: File handling needs backend configuration

### Signature
- **Use Case**: Patient consent, authorization
- **Note**: Signature pad integration needed

### Section Header
- **Use Case**: Group related fields
- **Note**: Visual separator only, no data collection

## Form Configuration

### Form Settings
- **Is Template**: Can be used as a template for other forms
- **Allow Multiple Submissions**: Whether users can submit multiple times
- **Require Approval**: Whether submissions need approval
- **Is Active**: Whether the form is currently active

### Access Control
- **Requires Permission**: Zanzibar permission required to access
- **Required Roles**: Roles that can access the form
- **Allowed Roles**: Specific roles allowed (overrides required roles)

### Entity Linking
- **Entity Type**: Type of entity this form is associated with (e.g., `patient`, `appointment`)
- **Entity ID**: ID of the specific entity (set when submitting)

## Using Forms

### Viewing a Form
Forms can be accessed via their slug: `/forms/{form-slug}`

### Submitting a Form
1. Navigate to the form URL
2. Fill out all required fields
3. Click "Submit Form"
4. Form data is saved and can be viewed in the admin panel

### Viewing Submissions
1. Navigate to `/admin/forms/{form-id}`
2. Click the "Submissions" tab
3. View all submissions with their data and status

## API Usage

### Create Form
```typescript
POST /api/v1/forms
{
  "form_name": "patient-intake",
  "form_slug": "patient-intake",
  "display_name": "Patient Intake Form",
  "module_name": "healthcare",
  "entity_type": "patient",
  "form_schema": [
    {
      "id": "field_1",
      "name": "first_name",
      "label": "First Name",
      "type": "text",
      "required": true
    }
  ]
}
```

### Submit Form
```typescript
POST /api/v1/forms/submit
{
  "form_definition_id": "uuid",
  "submission_data": {
    "first_name": "John",
    "last_name": "Doe"
  },
  "entity_type": "patient",
  "entity_id": "patient-uuid"
}
```

### Get Form by Slug
```typescript
GET /api/v1/forms/slug/{form-slug}
```

### List Forms
```typescript
GET /api/v1/forms?module_name=healthcare&is_active=true
```

## Best Practices

1. **Naming**: Use clear, descriptive names and slugs
2. **Field Names**: Use snake_case for field names (e.g., `first_name`)
3. **Validation**: Add appropriate validation rules
4. **Help Text**: Provide helpful guidance for users
5. **Required Fields**: Only mark truly required fields as required
6. **Organization**: Use section headers to group related fields
7. **Testing**: Preview forms before making them active
8. **Versioning**: Forms are automatically versioned when schema changes

## Example Forms

### Patient Intake Form
- Patient demographics
- Insurance information
- Emergency contact
- Medical history summary

### Vital Signs Form
- Temperature
- Blood pressure (systolic/diastolic)
- Heart rate
- Respiratory rate
- Oxygen saturation
- Pain scale

### Medication Administration Record
- Medication name
- Dosage
- Route
- Time given
- Given by (nurse name)
- Patient response

### Discharge Summary
- Discharge date/time
- Discharge diagnosis
- Treatment summary
- Medications prescribed
- Follow-up instructions
- Discharging physician

## Troubleshooting

### Form Not Appearing
- Check if form is active (`is_active: true`)
- Verify user has required permissions/roles
- Check form slug is correct

### Submission Failing
- Verify all required fields are filled
- Check validation rules
- Ensure form is active
- Verify user permissions

### Field Not Rendering
- Check field type is supported
- Verify field schema is valid JSON
- Check browser console for errors

## Future Enhancements

- Drag-and-drop field reordering
- Conditional field visibility
- Form templates library
- Form analytics and reporting
- PDF export of submissions
- Email notifications on submission
- Form version comparison
- Multi-language support

