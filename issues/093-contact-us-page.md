# Issue #093: Contact Us Page - Support and Feedback Form

**Priority:** High  
**Type:** Feature  
**Component:** Frontend + Backend  
**Labels:** `documentation`, `ui`, `support`

## Description

Create a "Contact Us" page with a contact form, support information, and multiple ways to reach the team. Enable users to report issues, request features, ask questions, and provide feedback.

## Current Behavior

- No contact page
- Users cannot reach the team
- No feedback mechanism
- No support channels listed
- Missing issue reporting

## Expected Behavior

- Contact form for inquiries
- Multiple contact methods
- Support information
- FAQ links
- Social media links
- Response time expectations
- Form validation and submission, emails should be sent to ndifrekemkpanam@gmail.com

## Affected Files

- **New file:** `frontend/src/app/contact/page.tsx`
- **New file:** `frontend/src/components/ContactForm.tsx`
- **New file:** `backend/src/api/contact.rs`
- **Update:** `frontend/src/components/layout/sidebar.tsx`

## Content Sections

### 1. Hero Section
- Page title
- Brief description
- Response time expectation

### 2. Contact Form
- Name (required)
- Email (required)
- Subject/Category (dropdown)
- Message (required)
- Attachment upload (optional)
- Submit button

### 3. Contact Methods
- Email address
- GitHub issues
- Discord/Telegram community
- Twitter/X handle
- LinkedIn page

### 4. Support Information
- Documentation links
- FAQ page link
- How to Use guide link
- API documentation link
- Status page link

### 5. Office/Location (Optional)
- Physical address (if applicable)
- Time zone
- Business hours

## UI Structure

```
┌─────────────────────────────────────┐
│ Contact Us                          │
├─────────────────────────────────────┤
│ Get in Touch                        │
│ We typically respond within 24 hours│
│                                     │
│ ┌─────────────────────────────────┐ │
│ │ Contact Form                    │ │
│ │                                 │ │
│ │ Name: [____________]            │ │
│ │ Email: [____________]           │ │
│ │ Category: [General Inquiry ▼]  │ │
│ │ Message:                        │ │
│ │ [________________________]      │ │
│ │ [________________________]      │ │
│ │ [________________________]      │ │
│ │                                 │ │
│ │ [Send Message]                  │ │
│ └─────────────────────────────────┘ │
│                                     │
│ Other Ways to Reach Us              │
│ ✉ support@stellar-insights.com     │
│ 🐙 GitHub Issues                    │
│ 💬 Discord Community                │
│ 🐦 @StellarInsights                 │
│                                     │
│ Need Help?                          │
│ → Check our FAQ                     │
│ → Read the How to Use guide         │
│ → View API Documentation            │
└─────────────────────────────────────┘
```

## Form Categories

- General Inquiry
- Technical Support
- Bug Report
- Feature Request
- Partnership Inquiry
- Press/Media
- Other

## Backend Implementation

### Contact Form Handler

```rust
#[derive(Debug, Deserialize, Validate)]
pub struct ContactFormData {
    #[validate(length(min = 2, max = 100))]
    pub name: String,
    
    #[validate(email)]
    pub email: String,
    
    pub category: ContactCategory,
    
    #[validate(length(min = 10, max = 5000))]
    pub message: String,
    
    pub attachment: Option<String>,
}

pub enum ContactCategory {
    GeneralInquiry,
    TechnicalSupport,
    BugReport,
    FeatureRequest,
    Partnership,
    Press,
    Other,
}
```

### Email Service Integration

```rust
pub struct EmailService {
    smtp_client: SmtpClient,
    from_address: String,
    to_address: String,
}

impl EmailService {
    pub async fn send_contact_form(&self, data: ContactFormData) -> Result<()>;
    pub async fn send_confirmation(&self, to: String) -> Result<()>;
}
```

## Acceptance Criteria

- [ ] Create Contact Us page layout
- [ ] Build contact form component
- [ ] Add form validation
- [ ] Implement backend endpoint
- [ ] Set up email service
- [ ] Send confirmation emails
- [ ] Store submissions in database
- [ ] Add rate limiting
- [ ] Display contact methods
- [ ] Add social media links
- [ ] Link support resources
- [ ] Make mobile-responsive
- [ ] Add CAPTCHA/spam protection
- [ ] Add success/error messages
- [ ] Link from footer and navigation
- [ ] Add tests and documentation

## Form Validation

### Frontend Validation
- Required fields
- Email format
- Message length (10-5000 chars)
- File size limits (if attachments)
- Real-time validation feedback

### Backend Validation
- Sanitize inputs
- Validate email format
- Check message length
- Rate limiting (5 submissions per hour per IP)
- Spam detection

## Email Templates

### Submission Notification (to team)
```
New Contact Form Submission

From: John Doe (john@example.com)
Category: Bug Report

Message:
[User's message here]

---
Submitted: 2026-02-19 10:30:00 UTC
IP: 192.168.1.1
```

### Confirmation Email (to user)
```
Thank you for contacting Stellar Insights!

We've received your message and will respond within 24 hours.

Your submission:
Category: Bug Report
Message: [First 200 chars...]

In the meantime, check out:
- FAQ: https://stellar-insights.com/faq
- Documentation: https://stellar-insights.com/docs

Best regards,
The Stellar Insights Team
```

## Database Schema

```sql
CREATE TABLE contact_submissions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    category TEXT NOT NULL,
    message TEXT NOT NULL,
    attachment_url TEXT,
    ip_address TEXT,
    user_agent TEXT,
    status TEXT DEFAULT 'pending',
    responded_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_status (status),
    INDEX idx_created_at (created_at)
);
```

## Implementation Steps

1. **Create Page Layout**:
   - Hero section
   - Form section
   - Contact methods section
   - Support links section

2. **Build Contact Form**:
   - Form fields
   - Validation
   - Submit handler
   - Success/error states

3. **Implement Backend**:
   - API endpoint
   - Email service
   - Database storage
   - Rate limiting

4. **Add Email Integration**:
   - SMTP configuration
   - Email templates
   - Confirmation emails

5. **Add Spam Protection**:
   - CAPTCHA (hCaptcha or reCAPTCHA)
   - Rate limiting
   - Input sanitization

6. **Testing**:
   - Form validation
   - Email delivery
   - Rate limiting
   - Mobile responsiveness

## Security Considerations

- Rate limiting to prevent spam
- Input sanitization
- CAPTCHA for bot protection
- Email validation
- File upload restrictions (if enabled)
- CSRF protection

## References

- Good examples:
  - Vercel Contact page
  - Linear Support page
  - Stripe Contact page

## Related Issues

- Related to: How to Use page
- Related to: About Us page
- Blocks: User support

## Estimated Effort

- Frontend form: 2 days
- Backend API: 1 day
- Email integration: 1 day
- Testing and polish: 1 day
- **Total: 5 days**
